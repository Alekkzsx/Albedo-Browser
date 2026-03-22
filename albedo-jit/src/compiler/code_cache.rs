//! # Code Cache (Tier 1 & 2)
//!
//! Armazena e gerencia o ciclo de vida do código de máquina gerado pelo JIT.
//! Indexado por `FunctionId`, permitindo lookup rápido e invalidação de código.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::engine::profiler::FunctionId;
use cranelift_module::FuncId;

/// Wrapper thread-safe para o ponteiro de código nativo.
/// SAFETY: O código JIT-compilado é imutável após a geração e reside em memória executável.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCodePtr(pub *const u8);

unsafe impl Send for NativeCodePtr {}
unsafe impl Sync for NativeCodePtr {}

/// Tiers de compilação do AlbedoJIT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitTier {
    /// Compilação rápida sem otimizações (Baseline).
    Baseline,
    /// Compilação otimizada com inferência de tipos (AlbedoTurbo).
    AlbedoTurbo,
}

/// Entrada no Code Cache contendo o código compilado e metadados.
pub struct CachedCode {
    pub func_id: FunctionId,
    /// Ponteiro para o código de máquina executável.
    pub native_ptr: NativeCodePtr,
    /// ID interno no módulo Cranelift.
    pub cranelift_id: FuncId,
    /// Tamanho do código em bytes.
    pub code_size_bytes: usize,
    /// Timestamp de quando foi compilado.
    pub compiled_at_ms: u64,
    /// Quantas vezes esta função JIT foi chamada.
    pub execution_count: AtomicU64,
    /// Nível de otimização (Tier).
    pub tier: JitTier,
    /// Define se a entrada ainda é válida (pode ser invalidada por mudança no fonte ou deopt).
    pub valid: AtomicBool,
}

impl CachedCode {
    pub fn new(
        func_id: FunctionId,
        native_ptr: NativeCodePtr,
        cranelift_id: FuncId,
        code_size_bytes: usize,
        tier: JitTier,
    ) -> Self {
        Self {
            func_id,
            native_ptr,
            cranelift_id,
            code_size_bytes,
            compiled_at_ms: current_timestamp_ms(),
            execution_count: AtomicU64::new(0),
            tier,
            valid: AtomicBool::new(true),
        }
    }

    pub fn increment_execution(&self) -> u64 {
        self.execution_count.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn is_valid(&self) -> bool {
        self.valid.load(Ordering::Acquire)
    }

    pub fn invalidate(&self) {
        self.valid.store(false, Ordering::Release);
    }
}

/// Snapshot das estatísticas de performance do Code Cache.
#[derive(Debug, Clone, Default)]
pub struct CacheStatsSnapshot {
    pub hits: u64,
    pub misses: u64,
    pub insertions: u64,
    pub invalidations: u64,
    pub total_code_bytes: u64,
}

/// Gerenciador central de código compilado.
pub struct CodeCache {
    entries: RwLock<HashMap<FunctionId, Arc<CachedCode>>>,
    hits: AtomicU64,
    misses: AtomicU64,
    insertions: AtomicU64,
    invalidations: AtomicU64,
    total_code_bytes: AtomicU64,
}

impl CodeCache {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            insertions: AtomicU64::new(0),
            invalidations: AtomicU64::new(0),
            total_code_bytes: AtomicU64::new(0),
        }
    }

    /// Busca uma função compilada no cache por seu FunctionId.
    /// Apenas retorna entradas válidas.
    pub fn lookup(&self, id: &FunctionId) -> Option<Arc<CachedCode>> {
        let entries = self.entries.read();
        if let Some(entry) = entries.get(id) {
            if entry.is_valid() {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Some(Arc::clone(entry));
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Insere ou atualiza uma entrada no cache.
    pub fn insert(&self, entry: CachedCode) {
        let size = entry.code_size_bytes as u64;
        let id = entry.func_id.clone();

        let mut entries = self.entries.write();
        entries.insert(id, Arc::new(entry));

        self.insertions.fetch_add(1, Ordering::Relaxed);
        self.total_code_bytes.fetch_add(size, Ordering::Relaxed);
    }

    /// Invalida uma entrada específica.
    pub fn invalidate(&self, id: &FunctionId) -> bool {
        let entries = self.entries.read();
        if let Some(entry) = entries.get(id) {
            if entry.is_valid() {
                entry.invalidate();
                self.invalidations.fetch_add(1, Ordering::Relaxed);
                return true;
            }
        }
        false
    }

    /// Verifica a existência de código válido para o ID (sem incrementar hits).
    pub fn contains(&self, id: &FunctionId) -> bool {
        let entries = self.entries.read();
        entries.get(id).map_or(false, |e| e.is_valid())
    }

    /// Retorna as estatísticas atuais do cache.
    pub fn stats(&self) -> CacheStatsSnapshot {
        CacheStatsSnapshot {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            insertions: self.insertions.load(Ordering::Relaxed),
            invalidations: self.invalidations.load(Ordering::Relaxed),
            total_code_bytes: self.total_code_bytes.load(Ordering::Relaxed),
        }
    }

    /// Retorna o número de entradas (incluindo inválidas no mapa, embora inacessíveis por lookup).
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Limpa completamente o cache.
    pub fn clear(&self) {
        let mut entries = self.entries.write();
        entries.clear();
        self.total_code_bytes.store(0, Ordering::Relaxed);
    }
}

static GLOBAL_CODE_CACHE: OnceLock<Arc<CodeCache>> = OnceLock::new();

pub fn set_global_code_cache(cache: Arc<CodeCache>) {
    let _ = GLOBAL_CODE_CACHE.set(cache);
}

pub fn get_global_code_cache() -> Arc<CodeCache> {
    GLOBAL_CODE_CACHE.get_or_init(|| Arc::new(CodeCache::new())).clone()
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_and_lookup() {
        let cache = CodeCache::new();
        let id = FunctionId("test_func".into());
        let dummy_ptr = NativeCodePtr(0x1234 as *const u8);
        let cranelift_id = FuncId::from_u32(0);

        let entry = CachedCode::new(id.clone(), dummy_ptr, cranelift_id, 128, JitTier::Baseline);

        cache.insert(entry);
        assert_eq!(cache.len(), 1);

        let found = cache.lookup(&id).expect("Deve encontrar a função");
        assert_eq!(found.native_ptr, dummy_ptr);
        assert_eq!(found.tier, JitTier::Baseline);
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn test_cache_miss() {
        let cache = CodeCache::new();
        let id = FunctionId("ghost".into());

        assert!(cache.lookup(&id).is_none());
        assert_eq!(cache.stats().misses, 1);
    }

    #[test]
    fn test_cache_invalidation() {
        let cache = CodeCache::new();
        let id = FunctionId("volatile".into());

        cache.insert(CachedCode::new(
            id.clone(),
            NativeCodePtr(0x1 as *const u8),
            FuncId::from_u32(0),
            64,
            JitTier::Baseline,
        ));

        assert!(cache.lookup(&id).is_some());
        assert!(cache.invalidate(&id));
        assert!(cache.lookup(&id).is_none()); // Agora é inválido
        assert_eq!(cache.stats().invalidations, 1);
        assert_eq!(cache.stats().misses, 1); // Lookup em entrada inválida conta como miss
    }

    #[test]
    fn test_execution_counter() {
        let id = FunctionId("counter".into());
        let entry = CachedCode::new(
            id,
            NativeCodePtr(0x1 as *const u8),
            FuncId::from_u32(0),
            10,
            JitTier::Baseline,
        );

        assert_eq!(entry.increment_execution(), 1);
        assert_eq!(entry.increment_execution(), 2);
        assert_eq!(entry.execution_count.load(Ordering::Relaxed), 2);
    }
}
