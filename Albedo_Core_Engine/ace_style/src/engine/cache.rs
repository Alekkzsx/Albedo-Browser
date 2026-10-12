//! # Cache de Compartilhamento de Estilo (`StyleSharingCache`)
//!
//! Reutilização em O(1) de instâncias `Arc<ComputedStyle>` entre elementos irmãos idênticos.

use crate::computed::style::ComputedStyle;
use ace_core::intern::Atom;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::sync::Arc;

/// Chave de assinatura para compartilhamento de estilo entre elementos irmãos.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StyleSharingKey {
    pub parent_ptr: usize,
    pub tag_name: Atom,
    pub id_attr: Option<Atom>,
    pub classes: Vec<Atom>,
    pub inline_style: Option<SmolStr>,
}

/// Cache de compartilhamento de estilo entre irmãos idênticos.
#[derive(Debug, Default)]
pub struct StyleSharingCache {
    entries: FxHashMap<StyleSharingKey, Arc<ComputedStyle>>,
}

impl StyleSharingCache {
    pub fn new() -> Self {
        Self {
            entries: FxHashMap::default(),
        }
    }

    /// Tenta recuperar uma instância compartilhada de `ComputedStyle`.
    pub fn try_get(&self, key: &StyleSharingKey) -> Option<Arc<ComputedStyle>> {
        self.entries.get(key).cloned()
    }

    /// Armazena um novo `ComputedStyle` associado à chave.
    pub fn insert(&mut self, key: StyleSharingKey, style: Arc<ComputedStyle>) {
        self.entries.insert(key, style);
    }

    /// Limpa o cache.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Quantidade de estilos compartilhados armazenados.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
