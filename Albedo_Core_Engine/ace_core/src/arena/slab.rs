//! # Arena Homogênea Geracional (SlotMap Pattern)
//!
//! Implementação de armazenamento denso e contíguo para nós DOM e árvores de Layout.
//! Cada elemento é referenciado por um [`ArenaId<T>`] composto por `(índice, versão)`.
//!
//! O versionamento é gerido individualmente por slot: a versão de um slot só é incrementada
//! quando o elemento que o ocupa é removido, maximizando a vida útil de cada identificador
//! e garantindo proteção estrita contra ponteiros soltos (*dangling handles*).

use crate::arena::id::ArenaId;
use crate::arena::stats::ArenaStats;
use std::mem;

/// Entrada interna da arena contendo valor e versão individual de geração.
#[derive(Debug, Clone)]
struct Entry<T> {
    version: u32,
    value: Option<T>,
}

/// Arena geracional com alocação densa, reutilização $O(1)$ de slots e verificação de versões.
#[derive(Debug, Clone)]
pub struct Arena<T> {
    /// Armazenamento denso de slots (ocupados e livres).
    entries: Vec<Entry<T>>,
    /// Índices de slots livres prontos para reutilização em $O(1)$.
    free_list: Vec<u32>,
    /// Total de alocações ao longo da vida da arena.
    total_allocated: u64,
    /// Total de liberações explícitas via [`remove`](Arena::remove) ou [`clear`](Arena::clear).
    total_freed: u64,
    /// Quantas vezes reutilizamos um slot livre em vez de crescer o `Vec`.
    slot_reuses: u64,
}

impl<T> Arena<T> {
    /// Cria uma arena vazia.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            free_list: Vec::new(),
            total_allocated: 0,
            total_freed: 0,
            slot_reuses: 0,
        }
    }

    /// Cria uma arena com capacidade pré-alocada para `capacity` valores.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let mut arena = Self::new();
        arena.entries.reserve(capacity);
        arena
    }

    /// Aloca `value` e retorna um [`ArenaId<T>`] estável.
    ///
    /// Reutiliza um slot livre da `free_list` quando houver; caso contrário, expande o vetor denso.
    #[inline]
    pub fn alloc(&mut self, value: T) -> ArenaId<T> {
        let (index, version) = match self.free_list.pop() {
            Some(idx) => {
                let entry = &mut self.entries[idx as usize];
                debug_assert!(entry.value.is_none(), "slot na free_list deve estar vazio");
                entry.value = Some(value);
                self.slot_reuses += 1;
                (idx, entry.version)
            }
            None => {
                let idx = self.entries.len() as u32;
                let version = 1;
                self.entries.push(Entry {
                    version,
                    value: Some(value),
                });
                (idx, version)
            }
        };

        self.total_allocated += 1;
        ArenaId::new(index, version)
    }

    /// Obtém `&T` para o identificador, ou `None` se ele for inválido/desalocado.
    #[inline]
    #[must_use]
    pub fn get(&self, id: ArenaId<T>) -> Option<&T> {
        let entry = self.entries.get(id.index() as usize)?;
        if entry.version != id.version() {
            return None;
        }
        entry.value.as_ref()
    }

    /// Obtém `&mut T` para o identificador, ou `None` se ele for inválido/desalocado.
    #[inline]
    #[must_use]
    pub fn get_mut(&mut self, id: ArenaId<T>) -> Option<&mut T> {
        let entry = self.entries.get_mut(id.index() as usize)?;
        if entry.version != id.version() {
            return None;
        }
        entry.value.as_mut()
    }

    /// Obtém `&T` sem verificações de limites para caminhos críticos de renderização onde o ID já foi validado.
    ///
    /// # Safety
    /// O identificador `id` deve pertencer a esta arena e estar vivo.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, id: ArenaId<T>) -> &T {
        let entry = self.entries.get_unchecked(id.index() as usize);
        debug_assert_eq!(entry.version, id.version());
        match &entry.value {
            Some(v) => v,
            None => std::hint::unreachable_unchecked(),
        }
    }

    /// Obtém `&mut T` sem verificações de limites para caminhos críticos onde o ID já foi validado.
    ///
    /// # Safety
    /// O identificador `id` deve pertencer a esta arena e estar vivo.
    #[inline(always)]
    pub unsafe fn get_unchecked_mut(&mut self, id: ArenaId<T>) -> &mut T {
        let entry = self.entries.get_unchecked_mut(id.index() as usize);
        debug_assert_eq!(entry.version, id.version());
        match &mut entry.value {
            Some(v) => v,
            None => std::hint::unreachable_unchecked(),
        }
    }

    /// Remove o valor referenciado, devolvendo-o. O identificador é invalidado imediatamente.
    #[inline]
    pub fn remove(&mut self, id: ArenaId<T>) -> Option<T> {
        let entry = self.entries.get_mut(id.index() as usize)?;
        if entry.version != id.version() {
            return None;
        }
        let value = entry.value.take();
        if value.is_some() {
            entry.version = next_version(entry.version);
            self.free_list.push(id.index());
            self.total_freed += 1;
        }
        value
    }

    /// `true` se o identificador aponta para um valor vivo.
    #[inline]
    #[must_use]
    pub fn contains(&self, id: ArenaId<T>) -> bool {
        self.get(id).is_some()
    }

    /// Número de valores vivos.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len() - self.free_list.len()
    }

    /// `true` se não houver nenhum valor vivo.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Descarta todos os valores, invalida todos os identificadores previamente emitidos
    /// e reconstrói a lista de slots livres de forma determinística sem duplicações.
    pub fn clear(&mut self) {
        let mut freed = 0u64;
        for entry in &mut self.entries {
            if entry.value.is_some() {
                entry.value = None;
                entry.version = next_version(entry.version);
                freed += 1;
            }
        }
        self.total_freed += freed;
        self.free_list.clear();
        self.free_list.extend((0..self.entries.len() as u32).rev());
    }

    /// Reserva capacidade para ao menos `additional` novos valores.
    pub fn reserve(&mut self, additional: usize) {
        self.entries.reserve(additional);
    }

    /// Capacidade atual do armazenamento (em slots).
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    /// Itera sobre os pares `(identificador, &valor)` de todos os valores vivos.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (ArenaId<T>, &T)> {
        self.entries.iter().enumerate().filter_map(|(i, entry)| {
            entry
                .value
                .as_ref()
                .map(|v| (ArenaId::new(i as u32, entry.version), v))
        })
    }

    /// Itera sobre os pares `(identificador, &mut valor)` de todos os valores vivos.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (ArenaId<T>, &mut T)> {
        self.entries
            .iter_mut()
            .enumerate()
            .filter_map(|(i, entry)| {
                entry
                    .value
                    .as_mut()
                    .map(|v| (ArenaId::new(i as u32, entry.version), v))
            })
    }

    /// Itera apenas sobre os valores vivos.
    #[inline]
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().filter_map(|e| e.value.as_ref())
    }

    /// Itera mutavelmente apenas sobre os valores vivos.
    #[inline]
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.entries.iter_mut().filter_map(|e| e.value.as_mut())
    }

    /// Varre a arena e remove todos os elementos para os quais `is_alive(id, &item)` retorna `false`.
    ///
    /// Ideal para implementar a fase de *Sweeping* de coletores de lixo (Mark-and-Sweep)
    /// ou coleta periódica de nós DOM desconectados e desalocados em uma única passagem $O(N)$.
    ///
    /// Retorna o número de elementos descartados.
    pub fn gc_sweep<F>(&mut self, mut is_alive: F) -> usize
    where
        F: FnMut(ArenaId<T>, &T) -> bool,
    {
        let mut swept = 0;
        for (i, entry) in self.entries.iter_mut().enumerate() {
            if let Some(val) = &entry.value {
                let id = ArenaId::new(i as u32, entry.version);
                if !is_alive(id, val) {
                    entry.value = None;
                    entry.version = next_version(entry.version);
                    self.free_list.push(i as u32);
                    swept += 1;
                }
            }
        }
        self.total_freed += swept as u64;
        swept
    }

    /// Filtra os elementos vivos da arena in-place, preservando apenas aqueles para os quais `f(&T)` retorna `true`.
    /// Retorna o número de elementos removidos.
    pub fn retain<F>(&mut self, mut f: F) -> usize
    where
        F: FnMut(&T) -> bool,
    {
        let mut removed = 0;
        for (i, entry) in self.entries.iter_mut().enumerate() {
            if let Some(val) = &entry.value {
                if !f(val) {
                    entry.value = None;
                    entry.version = next_version(entry.version);
                    self.free_list.push(i as u32);
                    removed += 1;
                }
            }
        }
        self.total_freed += removed as u64;
        removed
    }

    /// Reduz o tamanho do vetor subjacente eliminando slots livres consecutivos no final do armazenamento.
    pub fn shrink_to_fit(&mut self) {
        while let Some(last) = self.entries.last() {
            if last.value.is_none() {
                let last_idx = (self.entries.len() - 1) as u32;
                self.entries.pop();
                if let Some(pos) = self.free_list.iter().position(|&x| x == last_idx) {
                    self.free_list.swap_remove(pos);
                }
            } else {
                break;
            }
        }
        self.entries.shrink_to_fit();
        self.free_list.shrink_to_fit();
    }

    /// Coleta métricas de uso da arena para diagnóstico e telemetria.
    #[must_use]
    pub fn stats(&self) -> ArenaStats {
        ArenaStats {
            live: self.len(),
            capacity: self.entries.capacity(),
            total_allocated: self.total_allocated,
            total_freed: self.total_freed,
            slot_reuses: self.slot_reuses,
            bytes_allocated: self.len() * mem::size_of::<T>(),
        }
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Avança o contador de versão individual do slot, pulando o zero (reservado).
#[inline]
fn next_version(current: u32) -> u32 {
    let next = current.wrapping_add(1);
    if next == 0 {
        1
    } else {
        next
    }
}
