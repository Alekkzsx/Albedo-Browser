//! # Matched Properties Cache (MPC — Modelo Quantum CSS / Blink)
//!
//! Cache global indexado por assinatura de regras casadas e ponteiro de estilo do pai.
//! Permite saltar todo o processo de cascata em O(1) para elementos com o mesmo conjunto de regras.

use crate::computed::style::ComputedStyle;
use crate::parser::declaration_parser::ParsedDeclaration;
use crate::rule_tree::matcher::MatchedRule;
use core::hash::{Hash, Hasher};
use rustc_hash::{FxHashMap, FxHasher};
use std::sync::Arc;

/// Chave de assinatura para o Matched Properties Cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MpcKey {
    pub rules_hash: u64,
    pub parent_ptr: usize,
}

/// Cache global de resultados de cascata de propriedades.
#[derive(Debug, Default)]
pub struct MatchedPropertiesCache {
    entries: FxHashMap<MpcKey, Arc<ComputedStyle>>,
    hits: usize,
    misses: usize,
}

impl MatchedPropertiesCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Tenta recuperar o estilo computado correspondente à chave.
    pub fn get(&mut self, key: &MpcKey) -> Option<Arc<ComputedStyle>> {
        if let Some(style) = self.entries.get(key) {
            self.hits += 1;
            Some(Arc::clone(style))
        } else {
            self.misses += 1;
            None
        }
    }

    /// Insere uma nova resolução de estilo no cache.
    pub fn insert(&mut self, key: MpcKey, style: Arc<ComputedStyle>) {
        self.entries.insert(key, style);
    }

    /// Limpa todo o cache e reseta as estatísticas de acerto.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.hits = 0;
        self.misses = 0;
    }

    /// Retorna o número total de acertos (cache hits).
    pub fn hits(&self) -> usize {
        self.hits
    }

    /// Retorna o número total de falhas (cache misses).
    pub fn misses(&self) -> usize {
        self.misses
    }

    /// Retorna a taxa de acerto do cache (de 0.0 a 1.0).
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Retorna o número de entradas no cache.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Computa uma assinatura de hash `u64` estável combinando as regras casadas e declarações inline.
pub fn compute_rules_signature(
    matched_rules: &[MatchedRule],
    inline_decls: &[ParsedDeclaration],
) -> u64 {
    let mut hasher = FxHasher::default();
    matched_rules.len().hash(&mut hasher);
    for r in matched_rules {
        r.sheet_index.hash(&mut hasher);
        r.rule_index.hash(&mut hasher);
        r.selector_index.hash(&mut hasher);
        r.specificity.a.hash(&mut hasher);
        r.specificity.b.hash(&mut hasher);
        r.specificity.c.hash(&mut hasher);
    }
    inline_decls.len().hash(&mut hasher);
    for d in inline_decls {
        d.property.as_str().hash(&mut hasher);
        d.value.as_str().hash(&mut hasher);
        d.important.hash(&mut hasher);
    }
    hasher.finish()
}
