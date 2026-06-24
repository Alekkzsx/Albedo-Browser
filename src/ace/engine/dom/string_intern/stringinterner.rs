use super::*;
//! AceDOM String Interning System
//! Otimização crítica de memória para strings repetidas (tag names, attributes)
//! Reduz uso de memória em 60-80% para strings frequentes

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// Hasher rápido e seguro para string interning
use fxhash::FxBuildHasher;



/// Estrutura principal do interner
#[derive(Default)]
pub struct StringInterner {
    /// Mapa de string -> ID único
    map: InternMap,
    /// Vetor de strings armazenadas (por ID)
    strings: Vec<Arc<str>>,
    /// Estatísticas
    stats: InternStats,
}

impl StringInterner {
    /// Interns uma string, retornando um Arc compartilhado
    pub fn intern(&mut self, s: &str) -> Arc<str> {
        self.stats.total_interns += 1;
        // Verificar se já existe
        if let Some(&id) = self.map.get(s) {
            self.stats.cache_hits += 1;
            self.stats.memory_saved_bytes += s.len();
            return Arc::clone(&self.strings[id]);
        }

        // Nova string: alocar e registrar
        self.stats.cache_misses += 1;
        let arc = Arc::from(s);
        let id = self.strings.len();
        self.map.insert(Arc::clone(&arc), id);
        self.strings.push(Arc::clone(&arc));

        arc
    }

    /// Interns múltiplas strings de uma vez
    pub fn intern_many(&mut self, strings: &[&str]) -> Vec<Arc<str>> {
        strings.iter().map(|s| self.intern(s)).collect()
    }

    /// Obtém estatísticas atuais
    pub fn get_stats(&self) -> InternStats {
        self.stats.clone()
    }

    /// Limpa o cache (útil para testes ou reload de página)
    pub fn clear(&mut self) {
        self.map.clear();
        self.strings.clear();
        self.stats = InternStats::default();
    }

    /// Número de strings únicas armazenadas
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Verifica se está vazio
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}
