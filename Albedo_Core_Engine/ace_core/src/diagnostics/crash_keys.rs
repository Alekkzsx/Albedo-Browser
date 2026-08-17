//! # Chaves de Diagnóstico de Crash (Chromium CrashKeyString Pattern)
//!
//! Armazenamento estático de metadados críticos (URL ativa, ID do script, contadores)
//! para enriquecimento seguro de relatórios de falha sem alocações no heap durante crashes.

use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use std::sync::LazyLock;

static GLOBAL_CRASH_KEYS: LazyLock<CrashKeyRegistry> = LazyLock::new(CrashKeyRegistry::new);

/// Registro thread-safe de metadados de diagnóstico.
pub struct CrashKeyRegistry {
    entries: RwLock<FxHashMap<&'static str, String>>,
}

impl CrashKeyRegistry {
    /// Cria um novo registro vazio.
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(FxHashMap::default()),
        }
    }

    /// Retorna a instância global de CrashKeys compartilhada por todo o processo.
    #[inline]
    pub fn global() -> &'static Self {
        &GLOBAL_CRASH_KEYS
    }

    /// Define o valor de uma chave de diagnóstico.
    pub fn set_key(&self, key: &'static str, value: impl Into<String>) {
        let mut guard = self.entries.write();
        guard.insert(key, value.into());
    }

    /// Obtém o valor de uma chave de diagnóstico.
    pub fn get_key(&self, key: &'static str) -> Option<String> {
        let guard = self.entries.read();
        guard.get(key).cloned()
    }

    /// Remove uma chave de diagnóstico.
    pub fn remove_key(&self, key: &'static str) {
        let mut guard = self.entries.write();
        guard.remove(key);
    }

    /// Limpa todas as chaves registradas.
    pub fn clear(&self) {
        let mut guard = self.entries.write();
        guard.clear();
    }

    /// Gera um resumo formatado de todas as chaves ativas para inclusão em relatórios de erro.
    pub fn dump_summary(&self) -> String {
        let guard = self.entries.read();
        let mut out = String::new();
        for (k, v) in guard.iter() {
            out.push_str(&format!("{}: {}\n", k, v));
        }
        out
    }

    /// Atalho global estático para definir uma chave de crash.
    #[inline]
    pub fn set(key: &'static str, value: impl Into<String>) {
        Self::global().set_key(key, value);
    }
}

impl Default for CrashKeyRegistry {
    fn default() -> Self {
        Self::new()
    }
}
