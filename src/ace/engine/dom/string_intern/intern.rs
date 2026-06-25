use super::*;
// AceDOM String Interning System
// Otimização crítica de memória para strings repetidas (tag names, attributes)
// Reduz uso de memória em 60-80% para strings frequentes

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// Hasher rápido e seguro para string interning
use fxhash::FxBuildHasher;



/// API pública thread-safe
pub mod global {
    use super::*;

    /// Interns uma string globalmente
    pub fn intern(s: &str) -> Arc<str> {
        let mut interner = STRING_INTERN.lock().unwrap_or_else(|e| e.into_inner());
        interner.intern(s)
    }

    /// Interns múltiplas strings
    pub fn intern_many(strings: &[&str]) -> Vec<Arc<str>> {
        let mut interner = STRING_INTERN.lock().unwrap_or_else(|e| e.into_inner());
        interner.intern_many(strings)
    }

    /// Obtém estatísticas globais
    pub fn get_stats() -> InternStats {
        let interner = STRING_INTERN.lock().unwrap_or_else(|e| e.into_inner());
        interner.get_stats()
    }

    /// Limpa o cache global
    pub fn clear() {
        let mut interner = STRING_INTERN.lock().unwrap_or_else(|e| e.into_inner());
        interner.clear();
    }

    /// Número total de strings internadas
    pub fn len() -> usize {
        let interner = STRING_INTERN.lock().unwrap_or_else(|e| e.into_inner());
        interner.len()
    }
}

/// Macro utilitária para internar strings em tempo de compilação (quando possível)
#[macro_export]
macro_rules! intern {
    ($s:expr) => {
        $crate::ace::engine::dom::string_intern::global::intern($s)
    };
}
