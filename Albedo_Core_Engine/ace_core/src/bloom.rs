// ============================================================================
// Albedo Core Engine (ACE)
// File: bloom.rs
// Description: Bloom Filter (Filtro Probabilístico) baseado em bitset.
//              Otimização vital para Fast-Rejection de Seletores CSS O(1).
// Author: Albedo Browser Engineering Team
// ============================================================================

//! # Bloom Filter (Rejeição Ultra-Rápida)
//!
//! O WebKit e o Servo usam filtros de Bloom para acelerar drasticamente o
//! "Selector Matching". Quando perguntamos "O elemento X tem o ancestral Y?",
//! o filtro responde:
//! - "Com certeza não" (Fast-Rejection imediato sem percorrer a árvore DOM)
//! - "Talvez sim" (Faz a busca na árvore)
//!   Esta estrutura usa 2048 bits (256 bytes) e 2 hashes derivados por seletor.

const FILTER_SIZE_U64: usize = 32; // 32 * 64 = 2048 bits

/// Filtro Probabilístico de O(1) de custo e falsos-negativos absolutos = 0.
#[derive(Debug, Clone)]
pub struct BloomFilter {
    bits: [u64; FILTER_SIZE_U64],
}

impl BloomFilter {
    /// Cria um novo filtro vazio.
    pub fn new() -> Self {
        Self {
            bits: [0; FILTER_SIZE_U64],
        }
    }

    /// Limpa o filtro em O(1) simd-friendly.
    pub fn clear(&mut self) {
        self.bits.fill(0);
    }

    /// Deriva duas chaves (índices de bit) a partir de um hash u32.
    /// Técnica padrão "Double Hashing" extraída das propriedades do murmur3/fxhash.
    #[inline(always)]
    fn get_bit_indices(hash: u32) -> (usize, usize) {
        let bit1 = (hash & 2047) as usize; // Modulo 2048 rápido
        let bit2 = ((hash >> 16) & 2047) as usize;
        (bit1, bit2)
    }

    /// Insere um hash no filtro.
    #[inline]
    pub fn insert(&mut self, hash: u32) {
        let (b1, b2) = Self::get_bit_indices(hash);

        self.bits[b1 / 64] |= 1 << (b1 % 64);
        self.bits[b2 / 64] |= 1 << (b2 % 64);
    }

    /// Verifica se o hash PODE estar no filtro.
    /// Se retornar `false`, a resposta é **Garantidamente Não**.
    /// Se retornar `true`, a resposta é **Provavelmente Sim**.
    #[inline]
    pub fn might_contain(&self, hash: u32) -> bool {
        let (b1, b2) = Self::get_bit_indices(hash);

        let has_b1 = (self.bits[b1 / 64] & (1 << (b1 % 64))) != 0;
        let has_b2 = (self.bits[b2 / 64] & (1 << (b2 % 64))) != 0;

        has_b1 && has_b2
    }
}

impl Default for BloomFilter {
    fn default() -> Self {
        Self::new()
    }
}
