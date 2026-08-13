// ============================================================================
// Albedo Core Engine (ACE)
// File: hash.rs
// Description: FxHash nativo (Hashing ultra-rápido não-criptográfico).
//              Ideal para uso interno crítico (Maps/Sets do motor).
// Author: Albedo Browser Engineering Team
// ============================================================================

//! # Hashing de Ultra-Performance
//!
//! O `FxHash` (Firefox Hash / rustc-hash) é um algoritmo incrivelmente veloz,
//! otimizado para pequenos vetores e inteiros, rodando via multiplicações matemáticas
//! constantes sem a complexidade de algoritmos criptográficos como o `SipHash`
//! presente por padrão na biblioteca std do Rust.

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};
use std::ops::BitXor;

// A Constante Mágica do FxHash (Uma aproximação da proporção áurea)
const K: u64 = 0x517cc1b727220a95;

/// O Hasher ultra-veloz focado em performance pura de motor.
/// Não é resistente a colisões intencionais (HashDoS), devendo ser
/// estritamente utilizado em dados internos confiáveis.
#[derive(Default)]
pub struct FxHasher {
    hash: u64,
}

impl FxHasher {
    #[inline]
    fn add_to_hash(&mut self, i: u64) {
        self.hash = self.hash.rotate_left(5).bitxor(i).wrapping_mul(K);
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, mut bytes: &[u8]) {
        // Lemos os blocos em pedaços de 8, 4, 2 e 1 byte(s)
        while bytes.len() >= 8 {
            let mut chunk = [0u8; 8];
            chunk.copy_from_slice(&bytes[..8]);
            self.add_to_hash(u64::from_ne_bytes(chunk));
            bytes = &bytes[8..];
        }
        if bytes.len() >= 4 {
            let mut chunk = [0u8; 4];
            chunk.copy_from_slice(&bytes[..4]);
            self.add_to_hash(u32::from_ne_bytes(chunk) as u64);
            bytes = &bytes[4..];
        }
        if bytes.len() >= 2 {
            let mut chunk = [0u8; 2];
            chunk.copy_from_slice(&bytes[..2]);
            self.add_to_hash(u16::from_ne_bytes(chunk) as u64);
            bytes = &bytes[2..];
        }
        if let Some(&b) = bytes.first() {
            self.add_to_hash(b as u64);
        }
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add_to_hash(i as u64);
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.add_to_hash(i as u64);
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.add_to_hash(i as u64);
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.add_to_hash(i);
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add_to_hash(i as u64);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

/// O construtor do `FxHasher`.
pub type FxBuildHasher = BuildHasherDefault<FxHasher>;

/// `HashMap` otimizado em nível extremo para dados internos da engine.
/// CUIDADO: Vulnerável a HashDoS. NUNCA use com dados fornecidos pelo usuário.
pub type FxHashMap<K, V> = HashMap<K, V, FxBuildHasher>;

/// `HashSet` otimizado em nível extremo para dados internos da engine.
/// CUIDADO: Vulnerável a HashDoS.
pub type FxHashSet<T> = HashSet<T, FxBuildHasher>;

/// `HashMap` seguro para dados externos (Ex: Objetos JS, DOM Attributes).
/// Usa SipHash com chaves aleatórias (RandomState), imune a HashDoS em custo de leve perda de performance.
pub type SecureHashMap<K, V> = HashMap<K, V, std::collections::hash_map::RandomState>;

/// `HashSet` seguro para dados externos.
pub type SecureHashSet<T> = HashSet<T, std::collections::hash_map::RandomState>;

/// Calcula rapidamente um hash de 32 bits usando a lógica do FxHash.
/// Útil para estruturas que precisam de hashes 32 bits, como o Bloom Filter.
pub fn fxhash32(bytes: &[u8]) -> u32 {
    let mut hasher = FxHasher::default();
    hasher.write(bytes);
    let h64 = hasher.finish();
    (h64 as u32) ^ ((h64 >> 32) as u32)
}
