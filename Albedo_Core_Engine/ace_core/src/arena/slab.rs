//! Arena geracional homogênea — o armazenamento principal de nós.
//!
//! A [`Arena`] aloca valores de um único tipo `T` (por exemplo, o nó do DOM) em um
//! `Vec` denso e os referencia por [`ArenaId<T>`]. É a estrutura que permite que nós se
//! apontem mutuamente sem `Rc`, e que toda a árvore seja descartada em `O(1)` lógico.

use super::id::ArenaId;
use super::stats::ArenaStats;
use core::mem;

/// Um *slot* da arena: o valor em si + a versão usada para validar identificadores.
#[derive(Debug, Clone)]
struct Entry<T> {
    /// Versão global da arena no momento em que este *slot* foi ocupado.
    version: u32,
    /// O valor armazenado. `None` quando o *slot* está livre.
    value: Option<T>,
}

/// Arena geracional homogênea.
///
/// Aloca valores de um único tipo `T` e os referencia por [`ArenaId<T>`], um handle
/// `Copy` que pode ser guardado dentro de outros valores da mesma arena — o que torna
/// possível representar grafos cíclicos (como a árvore DOM) sem ponteiros posseiros.
///
/// # Complexidade
///
/// | Operação | Custo |
/// |---|---|
/// | [`alloc`](Arena::alloc) | `O(1)` amortizado |
/// | [`get`](Arena::get) / [`get_mut`](Arena::get_mut) | `O(1)` |
/// | [`remove`](Arena::remove) | `O(1)` |
/// | [`clear`](Arena::clear) | `O(n)` para drop dos valores, `O(1)` lógico para o DOM |
///
/// # Exemplo
///
/// ```
/// use ace_core::arena::{Arena, ArenaId};
///
/// let mut arena: Arena<i32> = Arena::new();
/// let a = arena.alloc(10);
/// let b = arena.alloc(20);
///
/// assert_eq!(arena.get(a), Some(&10));
/// assert_eq!(arena.get(b), Some(&20));
///
/// arena.remove(a);
/// assert_eq!(arena.get(a), None); // identificador invalidado
/// assert_eq!(arena.get(b), Some(&20)); // `b` segue válido
/// ```
#[derive(Debug, Clone)]
pub struct Arena<T> {
    /// Armazenamento denso de *slots* (ocupados e livres).
    entries: Vec<Entry<T>>,
    /// Índices de *slots* livres, prontos para reutilização em `O(1)`.
    free_list: Vec<u32>,
    /// Contador de versão global, monotônico. **Nunca é zerado** — nem no `clear` —
    /// para que identificadores de páginas anteriores sejam sempre inválidos.
    version: u32,
    /// Total de alocações ao longo da vida da arena.
    total_allocated: u64,
    /// Total de liberações explícitas via [`remove`](Arena::remove).
    total_freed: u64,
    /// Quantas vezes reutilizamos um *slot* livre em vez de crescer o `Vec`.
    slot_reuses: u64,
}

impl<T> Arena<T> {
    /// Cria uma arena vazia.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            free_list: Vec::new(),
            version: 0,
            total_allocated: 0,
            total_freed: 0,
            slot_reuses: 0,
        }
    }

    /// Cria uma arena com capacidade pré-alocada para `capacity` valores.
    ///
    /// Recomendado no carregamento de páginas: estimar o número de nós evita
    /// *reallocs* sucessivos do `Vec` durante o *parsing*.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let mut arena = Self::new();
        arena.entries.reserve(capacity);
        arena
    }

    /// Aloca `value` e retorna um [`ArenaId<T>`] estável.
    ///
    /// Reutiliza um *slot* livre quando houver; caso contrário, cresce o armazenamento.
    /// O identificador retornado permanece válido até [`remove`](Arena::remove) ou
    /// [`clear`](Arena::clear).
    #[inline]
    pub fn alloc(&mut self, value: T) -> ArenaId<T> {
        self.version = next_version(self.version);
        let version = self.version;

        let index = match self.free_list.pop() {
            Some(idx) => {
                let entry = &mut self.entries[idx as usize];
                debug_assert!(entry.value.is_none(), "slot na free_list deve estar vazio");
                entry.version = version;
                entry.value = Some(value);
                self.slot_reuses += 1;
                idx
            }
            None => {
                let idx = self.entries.len() as u32;
                self.entries.push(Entry {
                    version,
                    value: Some(value),
                });
                idx
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

    /// Remove o valor referenciado, devolvendo-o. O identificador é invalidado.
    ///
    /// Chamadas repetidas com o mesmo identificador retornam `None` (não há *double free*).
    #[inline]
    pub fn remove(&mut self, id: ArenaId<T>) -> Option<T> {
        let entry = self.entries.get_mut(id.index() as usize)?;
        if entry.version != id.version() {
            return None;
        }
        let value = entry.value.take();
        if value.is_some() {
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

    /// Descarta todos os valores.
    ///
    /// O contador de versão **não** é reiniciado: qualquer [`ArenaId`] emitido antes do
    /// `clear` continuará inválido mesmo após o armazenamento ser reutilizado.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.free_list.clear();
    }

    /// Reserva capacidade para ao menos `additional` novos valores.
    pub fn reserve(&mut self, additional: usize) {
        self.entries.reserve(additional);
    }

    /// Capacidade atual do armazenamento (em *slots*).
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    /// Itera sobre os pares `(identificador, &valor)` de todos os valores vivos.
    ///
    /// Este é o ponto de integração com o coletor de ciclos do `ace_js` (Fase 10).
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

    /// Coleta métricas de uso — alimenta o *profiling* de memória do projeto.
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

/// Avança o contador de versão, pulando o zero (reservado como "inválido").
///
/// O *wrap-around* de um `u32` só ocorreria após ~4 bilhões de alocações na **mesma**
/// arena — irrealista para uma aba de navegador. Documentado como limite teórico.
#[inline]
fn next_version(current: u32) -> u32 {
    let next = current.wrapping_add(1);
    if next == 0 {
        1
    } else {
        next
    }
}
