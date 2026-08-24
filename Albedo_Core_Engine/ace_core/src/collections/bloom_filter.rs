//! # Filtro Bloom Probabilístico em Stack
//!
//! Estrutura de dados probabilística zero-allocation de alta velocidade para testes de pertinência em $O(1)$.
//! Utilizada pelo motor CSS (`ace_style`) para rejeição instantânea de seletores de ancestrais não correspondentes.

use crate::utils::fast_hash;

/// Filtro Bloom com tamanho fixo em bits (alocado em stack), com suporte a 4 funções de dispersão via double hashing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BloomFilter<const WORDS: usize> {
    words: [u64; WORDS],
}

impl<const WORDS: usize> BloomFilter<WORDS> {
    /// Número total de bits suportados pelo filtro.
    pub const BITS: usize = WORDS * 64;

    /// Cria um filtro Bloom vazio.
    #[inline]
    pub const fn new() -> Self {
        Self { words: [0; WORDS] }
    }

    /// Limpa todos os bits do filtro.
    #[inline]
    pub fn clear(&mut self) {
        self.words.fill(0);
    }

    /// Retorna `true` se nenhum elemento foi inserido.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|&w| w == 0)
    }

    /// Conta quantos bits estão marcados como 1.
    #[inline]
    pub fn count_ones(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// Insere um valor de hash de 64 bits no filtro com 4 passos desenrolados e indexação sem divisão.
    #[inline(always)]
    pub fn insert_hash(&mut self, hash: u64) {
        let (h1, h2) = Self::split_hash(hash);
        let is_power_of_two = Self::BITS.is_power_of_two();
        let mask = Self::BITS.wrapping_sub(1);

        macro_rules! set_step {
            ($step:expr) => {
                let h = h1.wrapping_add($step * h2) as usize;
                let bit_idx = if is_power_of_two { h & mask } else { h % Self::BITS };
                let word_idx = bit_idx >> 6;
                let bit_in_word = bit_idx & 63;
                self.words[word_idx] |= 1u64 << bit_in_word;
            };
        }

        set_step!(0);
        set_step!(1);
        set_step!(2);
        set_step!(3);
    }

    /// Insere uma string no filtro.
    #[inline(always)]
    pub fn insert_str(&mut self, text: &str) {
        let hash = fast_hash(text);
        self.insert_hash(hash);
    }

    /// Consulta se um valor de hash possivelmente está no conjunto com early-exit no primeiro bit 0.
    #[inline(always)]
    pub fn contains_hash(&self, hash: u64) -> bool {
        let (h1, h2) = Self::split_hash(hash);
        let is_power_of_two = Self::BITS.is_power_of_two();
        let mask = Self::BITS.wrapping_sub(1);

        macro_rules! check_step {
            ($step:expr) => {
                let h = h1.wrapping_add($step * h2) as usize;
                let bit_idx = if is_power_of_two { h & mask } else { h % Self::BITS };
                let word_idx = bit_idx >> 6;
                let bit_in_word = bit_idx & 63;
                if (self.words[word_idx] & (1u64 << bit_in_word)) == 0 {
                    return false;
                }
            };
        }

        check_step!(0);
        check_step!(1);
        check_step!(2);
        check_step!(3);

        true
    }

    /// Consulta se uma string possivelmente está no conjunto.
    #[inline]
    pub fn contains_str(&self, text: &str) -> bool {
        let hash = fast_hash(text);
        self.contains_hash(hash)
    }

    /// Divide um hash de 64 bits em duas partes independentes de 32 bits (Double Hashing).
    #[inline]
    fn split_hash(hash: u64) -> (u64, u64) {
        let h1 = hash & 0xFFFF_FFFF;
        let mut h2 = hash >> 32;
        if h2 == 0 {
            h2 = 0x9E37_79B9; // Constante de dispersão áurea
        }
        (h1, h2)
    }
}

impl<const WORDS: usize> Default for BloomFilter<WORDS> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<const WORDS: usize> std::fmt::Debug for BloomFilter<WORDS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BloomFilter")
            .field("bits", &Self::BITS)
            .field("active_bits", &self.count_ones())
            .finish()
    }
}
