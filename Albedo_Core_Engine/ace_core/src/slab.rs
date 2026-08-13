// ============================================================================
// Albedo Core Engine (ACE)
// File: slab.rs
// Description: Slab Allocator (Pool Tipado de objetos com reúso de slots em O(1)).
//              Essencial para nodes do DOM que nascem e morrem dinamicamente.
// Author: Albedo Browser Engineering Team
// ============================================================================

//! # Slab Allocator (Memória Dinâmica O(1))
//! 
//! Uma `Arena` (Bump Allocator) é O(1) para alocar, mas os objetos não podem ser
//! liberados individualmente. O `Slab` resolve isso: um vetor pré-alocado que
//! encadeia os slots vazios numa *free-list*.
//! Ao remover um elemento, seu índice passa a apontar para o próximo slot vazio,
//! permitindo que a próxima inserção reutilize a memória O(1) sem *fragmentação*.

pub enum Entry<T> {
    Occupied(T),
    Vacant(usize),
}

pub struct Slab<T> {
    entries: Vec<Entry<T>>,
    /// O índice do primeiro slot vazio na free-list.
    next_free: usize,
    len: usize,
}

impl<T> Slab<T> {
    /// Cria um novo Slab vazio.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_free: 0,
            len: 0,
        }
    }

    /// Cria um Slab reservando capacidade inicial no Heap.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            next_free: 0,
            len: 0,
        }
    }

    /// Retorna a quantidade de elementos vivos no Slab.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Verifica se o Slab está vazio.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Insere um valor e retorna o seu ID (índice absoluto). Operação O(1).
    pub fn insert(&mut self, val: T) -> usize {
        let key = self.next_free;
        if key == self.entries.len() {
            // A free-list acabou, o vetor precisa crescer
            self.entries.push(Entry::Occupied(val));
            self.next_free = key + 1;
        } else {
            // Reutiliza um slot existente, lendo quem era o próximo vazio
            match self.entries[key] {
                Entry::Vacant(next) => {
                    self.next_free = next;
                }
                Entry::Occupied(_) => {
                    unreachable!("Slab corrompido: next_free apontava para slot ocupado");
                }
            }
            self.entries[key] = Entry::Occupied(val);
        }
        self.len += 1;
        key
    }

    /// Remove e retorna o valor no índice especificado. Operação O(1).
    /// Pânico: Se o índice for inválido ou o slot já estiver vazio.
    pub fn remove(&mut self, key: usize) -> T {
        assert!(key < self.entries.len(), "Slab key fora dos limites");
        
        // Troca temporariamente o valor por Vacant para podermos retornar `T` (ownership)
        let old_entry = std::mem::replace(&mut self.entries[key], Entry::Vacant(self.next_free));
        
        match old_entry {
            Entry::Occupied(val) => {
                // Atualiza a head da free-list
                self.next_free = key;
                self.len -= 1;
                val
            }
            Entry::Vacant(_) => {
                panic!("Slab double free: Tentativa de remover slot já vazio no índice {}", key);
            }
        }
    }

    /// Recupera uma referência imutável ao valor. O(1).
    #[inline]
    pub fn get(&self, key: usize) -> Option<&T> {
        if let Some(Entry::Occupied(ref val)) = self.entries.get(key) {
            Some(val)
        } else {
            None
        }
    }

    /// Recupera uma referência mutável ao valor. O(1).
    #[inline]
    pub fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        if let Some(Entry::Occupied(ref mut val)) = self.entries.get_mut(key) {
            Some(val)
        } else {
            None
        }
    }
}

impl<T> Default for Slab<T> {
    fn default() -> Self {
        Self::new()
    }
}
