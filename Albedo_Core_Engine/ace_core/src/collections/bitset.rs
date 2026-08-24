//! # BitSets de Alta Performance para Flags de Nós e Dirty Tracking
//!
//! Estruturas de manipulação de bits em stack (zero alocações no heap) para rastreamento
//! ultra-rápido de propriedades sujas (dirty layout/style) e pseudo-classes de nós do DOM.

use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};
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
    #[inline(always)]
    pub fn set(&mut self, bit: usize, val: bool) {
        if bit < Self::capacity() {
            let word_idx = bit >> 6;
            let bit_mask = 1u64 << (bit & 63);
            if val {
                self.words[word_idx] |= bit_mask;
            } else {
                self.words[word_idx] &= !bit_mask;
            }
        }
    }

    /// Consulta se um bit específico está ativo (`true`).
    #[inline(always)]
    pub fn get(&self, bit: usize) -> bool {
        if bit < Self::capacity() {
            let word_idx = bit >> 6;
            let bit_mask = 1u64 << (bit & 63);
            (self.words[word_idx] & bit_mask) != 0
        } else {
            false
        }
    }

    /// Zera todos os bits.
    #[inline(always)]
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

    /// Itera sobre os índices de todos os bits ativos (`1`) em tempo O(popcount) via instruções de CPU `trailing_zeros`.
    pub fn ones(&self) -> OnesIter<WORDS> {
        OnesIter::new(self)
    }
}

impl<const WORDS: usize> Default for FixedBitSet<WORDS> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<const WORDS: usize> BitAnd for FixedBitSet<WORDS> {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        let mut result = Self::new();
        for i in 0..WORDS {
            result.words[i] = self.words[i] & rhs.words[i];
        }
        result
    }
}

impl<const WORDS: usize> BitAndAssign for FixedBitSet<WORDS> {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        for i in 0..WORDS {
            self.words[i] &= rhs.words[i];
        }
    }
}

impl<const WORDS: usize> BitOr for FixedBitSet<WORDS> {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        let mut result = Self::new();
        for i in 0..WORDS {
            result.words[i] = self.words[i] | rhs.words[i];
        }
        result
    }
}

impl<const WORDS: usize> BitOrAssign for FixedBitSet<WORDS> {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        for i in 0..WORDS {
            self.words[i] |= rhs.words[i];
        }
    }
}

impl<const WORDS: usize> BitXor for FixedBitSet<WORDS> {
    type Output = Self;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        let mut result = Self::new();
        for i in 0..WORDS {
            result.words[i] = self.words[i] ^ rhs.words[i];
        }
        result
    }
}

impl<const WORDS: usize> BitXorAssign for FixedBitSet<WORDS> {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Self) {
        for i in 0..WORDS {
            self.words[i] ^= rhs.words[i];
        }
    }
}

impl<const WORDS: usize> Not for FixedBitSet<WORDS> {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        let mut result = Self::new();
        for i in 0..WORDS {
            result.words[i] = !self.words[i];
        }
        result
    }
}

impl<const WORDS: usize> From<[u64; WORDS]> for FixedBitSet<WORDS> {
    #[inline]
    fn from(words: [u64; WORDS]) -> Self {
        Self { words }
    }
}

impl<const WORDS: usize> From<FixedBitSet<WORDS>> for [u64; WORDS] {
    #[inline]
    fn from(bitset: FixedBitSet<WORDS>) -> Self {
        bitset.words
    }
}

impl From<u64> for FixedBitSet<1> {
    #[inline]
    fn from(val: u64) -> Self {
        Self { words: [val] }
    }
}

/// Iterador sobre os índices de bits ativos em um `FixedBitSet` (otimizado via hardware intrinsics `trailing_zeros`).
pub struct OnesIter<const WORDS: usize> {
    words: [u64; WORDS],
    current_word_idx: usize,
    current_word: u64,
}

impl<const WORDS: usize> OnesIter<WORDS> {
    fn new(bitset: &FixedBitSet<WORDS>) -> Self {
        let current_word = if WORDS > 0 { bitset.words[0] } else { 0 };
        Self {
            words: bitset.words,
            current_word_idx: 0,
            current_word,
        }
    }
}

impl<const WORDS: usize> Iterator for OnesIter<WORDS> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.current_word == 0 {
            self.current_word_idx += 1;
            if self.current_word_idx >= WORDS {
                return None;
            }
            self.current_word = self.words[self.current_word_idx];
        }

        let bit_in_word = self.current_word.trailing_zeros() as usize;
        let global_bit = self.current_word_idx * 64 + bit_in_word;

        // Limpa o bit menos significativo ativo (1 ciclo de clock: w &= w - 1)
        self.current_word &= self.current_word.wrapping_sub(1);

        Some(global_bit)
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
