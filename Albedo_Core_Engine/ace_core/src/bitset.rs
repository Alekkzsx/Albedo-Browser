// ============================================================================
// Albedo Core Engine (ACE)
// File: bitset.rs
// Description: Vetor de bits dinâmico e hiper-compacto (64 bits por bloco).
//              Essencial para rastreamento de flags massivas (CSS Selectors, ECS).
// Author: Albedo Browser Engineering Team
// ============================================================================

//! # BitSet: Vetor de Bits Compacto
//! 
//! Substitui `Vec<bool>` para garantir um consumo de memória 64x menor.
//! Permite operações lógicas matemáticas de conjunto em massa (`Union`, `Intersection`)
//! de forma praticamente instantânea através de instruções SIMD/Bitwise do processador.

use std::cmp;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BitSet {
    blocks: Vec<u64>,
}

impl BitSet {
    /// Cria um novo BitSet vazio.
    #[inline]
    pub fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    /// Cria um novo BitSet reservando capacidade para pelo menos `capacity` bits.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            blocks: Vec::with_capacity((capacity + 63) / 64),
        }
    }

    #[inline]
    fn ensure_capacity_for(&mut self, bit_idx: usize) {
        let block_idx = bit_idx / 64;
        if block_idx >= self.blocks.len() {
            self.blocks.resize(block_idx + 1, 0);
        }
    }

    /// Ativa o bit no índice fornecido (setta para 1).
    #[inline]
    pub fn insert(&mut self, bit_idx: usize) {
        self.ensure_capacity_for(bit_idx);
        let block_idx = bit_idx / 64;
        let bit_offset = bit_idx % 64;
        self.blocks[block_idx] |= 1 << bit_offset;
    }

    /// Desativa o bit no índice fornecido (setta para 0).
    #[inline]
    pub fn remove(&mut self, bit_idx: usize) {
        let block_idx = bit_idx / 64;
        if block_idx < self.blocks.len() {
            let bit_offset = bit_idx % 64;
            self.blocks[block_idx] &= !(1 << bit_offset);
        }
    }

    /// Verifica se o bit no índice fornecido está ativado.
    #[inline]
    pub fn contains(&self, bit_idx: usize) -> bool {
        let block_idx = bit_idx / 64;
        if block_idx < self.blocks.len() {
            let bit_offset = bit_idx % 64;
            (self.blocks[block_idx] & (1 << bit_offset)) != 0
        } else {
            false
        }
    }

    /// Retorna o número total de bits ativados (Population Count / popcnt).
    /// Altamente otimizado pelo hardware (`_popcnt64`).
    #[inline]
    pub fn count_ones(&self) -> u32 {
        self.blocks.iter().map(|b| b.count_ones()).sum()
    }

    /// Modifica este BitSet para conter a União matemática com outro BitSet (In-Place OR).
    #[inline]
    pub fn union_with(&mut self, other: &Self) {
        let max_len = cmp::max(self.blocks.len(), other.blocks.len());
        self.blocks.resize(max_len, 0);
        for (a, b) in self.blocks.iter_mut().zip(other.blocks.iter()) {
            *a |= *b;
        }
    }

    /// Modifica este BitSet para conter a Interseção com outro BitSet (In-Place AND).
    #[inline]
    pub fn intersection_with(&mut self, other: &Self) {
        // Reduzimos o tamanho para o menor dos dois, já que os blocos além não podem ter intersecção
        self.blocks.truncate(other.blocks.len());
        for (a, b) in self.blocks.iter_mut().zip(other.blocks.iter()) {
            *a &= *b;
        }
    }

    /// Limpa todos os bits (setta tudo para 0) mantendo a capacidade alocada.
    #[inline]
    pub fn clear(&mut self) {
        for block in self.blocks.iter_mut() {
            *block = 0;
        }
    }
}
