//! # BitSets de Alta Performance para Flags de Nós e Dirty Tracking
//!
//! Estruturas de manipulação de bits em stack (zero alocações no heap) para rastreamento
//! ultra-rápido de propriedades sujas (dirty layout/style) e pseudo-classes de nós do DOM.

use std::sync::atomic::{AtomicU64, Ordering};

/// BitSet de tamanho fixo em stack, parametrizado pelo número de palavras `u64`.
///
/// Ex: `FixedBitSet<2>` armazena 128 bits; `FixedBitSet<4>` armazena 256 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FixedBitSet<const WORDS: usize> {
    words: [u64; WORDS],
}

impl<const WORDS: usize> FixedBitSet<WORDS> {
    /// Cria um BitSet vazio (todos os bits zerados).
    #[inline]
    pub const fn new() -> Self {
        Self { words: [0; WORDS] }
    }

    /// Capacidade total de bits suportada por este BitSet.
    #[inline]
    pub const fn capacity() -> usize {
        WORDS * 64
    }

    /// Define o estado de um bit específico (`true` ou `false`).
    #[inline]
    pub fn set(&mut self, bit: usize, val: bool) {
        if bit < Self::capacity() {
            let word_idx = bit / 64;
            let bit_idx = bit % 64;
            if val {
                self.words[word_idx] |= 1 << bit_idx;
            } else {
                self.words[word_idx] &= !(1 << bit_idx);
            }
        }
    }

    /// Consulta se um bit específico está ativo (`true`).
    #[inline]
    pub fn get(&self, bit: usize) -> bool {
        if bit < Self::capacity() {
            let word_idx = bit / 64;
            let bit_idx = bit % 64;
            (self.words[word_idx] & (1 << bit_idx)) != 0
        } else {
            false
        }
    }

    /// Zera todos os bits.
    #[inline]
    pub fn clear(&mut self) {
        self.words.fill(0);
    }

    /// Retorna `true` se todos os bits forem zero.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|&w| w == 0)
    }

    /// Conta quantos bits estão ativos (`1`).
    #[inline]
    pub fn count_ones(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }
}

impl<const WORDS: usize> Default for FixedBitSet<WORDS> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// BitSet concorrente e lock-free baseado em `AtomicU64`.
///
/// Permite que múltiplas threads sinalizem flags de invalidação de nós DOM sem locks.
pub struct AtomicBitSet<const WORDS: usize> {
    words: [AtomicU64; WORDS],
}

impl<const WORDS: usize> AtomicBitSet<WORDS> {
    /// Cria um BitSet atômico vazio.
    pub fn new() -> Self {
        Self {
            // Inicializa o array de AtomicU64
            words: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }

    /// Capacidade total de bits.
    #[inline]
    pub const fn capacity() -> usize {
        WORDS * 64
    }

    /// Ativa ou desativa um bit atomicamente.
    #[inline]
    pub fn set(&self, bit: usize, val: bool, order: Ordering) {
        if bit < Self::capacity() {
            let word_idx = bit / 64;
            let bit_idx = bit % 64;
            let mask = 1u64 << bit_idx;
            if val {
                self.words[word_idx].fetch_or(mask, order);
            } else {
                self.words[word_idx].fetch_and(!mask, order);
            }
        }
    }

    /// Lê o valor de um bit atomicamente.
    #[inline]
    pub fn get(&self, bit: usize, order: Ordering) -> bool {
        if bit < Self::capacity() {
            let word_idx = bit / 64;
            let bit_idx = bit % 64;
            let mask = 1u64 << bit_idx;
            (self.words[word_idx].load(order) & mask) != 0
        } else {
            false
        }
    }

    /// Ativa um bit e retorna o valor que ele possuía anteriormente (Test-and-Set atômico).
    #[inline]
    pub fn test_and_set(&self, bit: usize, order: Ordering) -> bool {
        if bit < Self::capacity() {
            let word_idx = bit / 64;
            let bit_idx = bit % 64;
            let mask = 1u64 << bit_idx;
            (self.words[word_idx].fetch_or(mask, order) & mask) != 0
        } else {
            false
        }
    }

    /// Zera todos os bits atomicamente.
    pub fn clear(&self, order: Ordering) {
        for word in &self.words {
            word.store(0, order);
        }
    }
}

impl<const WORDS: usize> Default for AtomicBitSet<WORDS> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const WORDS: usize> std::fmt::Debug for AtomicBitSet<WORDS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AtomicBitSet")
            .field("capacity", &Self::capacity())
            .finish()
    }
}
