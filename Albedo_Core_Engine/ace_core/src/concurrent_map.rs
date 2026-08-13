// ============================================================================
// Albedo Core Engine (ACE)
// File: concurrent_map.rs
// Description: Sharded Concurrent HashMap Lock-Free(ish).
//              Divide a contenção de acesso usando N buckets protegidos
//              por SpinLocks independentes. Vital para Caches Globais (DNS, CSS).
// Author: Albedo Browser Engineering Team
// ============================================================================

use crate::hash::FxHasher;
use crate::sync::SpinLock;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hash, Hasher};

/// Número de Shards. Deve ser potência de 2 para otimização de módulo.
const SHARD_COUNT: usize = 32;
const SHARD_MASK: u64 = (SHARD_COUNT - 1) as u64;

type FxBuildHasher = BuildHasherDefault<FxHasher>;

/// Um mapa hash concorrente fragmentado.
/// Múltiplas threads podem ler e escrever simultaneamente desde que não
/// colidam no mesmo "Shard".
pub struct ConcurrentMap<K, V> {
    shards: [SpinLock<HashMap<K, V, FxBuildHasher>>; SHARD_COUNT],
}

impl<K: Eq + Hash + Clone, V: Clone> ConcurrentMap<K, V> {
    pub fn new() -> Self {
        Self {
            shards: std::array::from_fn(|_| SpinLock::new(HashMap::default())),
        }
    }

    /// Calcula o shard ideal para a chave.
    #[inline]
    fn shard_idx(&self, key: &K) -> usize {
        let mut hasher = FxHasher::default();
        key.hash(&mut hasher);
        (hasher.finish() & SHARD_MASK) as usize
    }

    /// Insere uma chave/valor no mapa.
    pub fn insert(&self, key: K, value: V) {
        let idx = self.shard_idx(&key);
        let mut shard = self.shards[idx].lock();
        shard.insert(key, value);
    }

    /// Tenta obter o valor associado à chave. Retorna uma cópia do valor se existir.
    pub fn get(&self, key: &K) -> Option<V> {
        let idx = self.shard_idx(key);
        let shard = self.shards[idx].lock();
        shard.get(key).cloned()
    }

    /// Obtém um valor, ou insere o resultado da factory lockando apenas durante a inserção.
    pub fn get_or_insert_with<F>(&self, key: K, factory: F) -> V
    where
        F: FnOnce() -> V,
    {
        let idx = self.shard_idx(&key);
        let mut shard = self.shards[idx].lock();

        if let Some(val) = shard.get(&key) {
            return val.clone();
        }

        let new_val = factory();
        shard.insert(key, new_val.clone());
        new_val
    }

    /// Remove a chave do mapa e retorna o valor antigo, se houver.
    pub fn remove(&self, key: &K) -> Option<V> {
        let idx = self.shard_idx(key);
        let mut shard = self.shards[idx].lock();
        shard.remove(key)
    }
}

impl<K: Eq + Hash + Clone, V: Clone> Default for ConcurrentMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
