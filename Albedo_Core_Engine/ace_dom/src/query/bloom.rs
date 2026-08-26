//! # Counting Bloom Filter para Ancestrais (AncestorFilter — Estilo Blink/WebKit)
//!
//! Filtro de Bloom de Contagem em $O(1)$ para rejeição rápida de seletores CSS descendentes
//! sem efetuar subidas custosas na árvore DOM.

use crate::node::ElementData;
use crate::query::selector::{CompoundSelector, SimpleSelector};
use ace_core::intern::Atom;

/// Filtro de Bloom de contagem de 64 buckets para ancestrais.
#[derive(Debug, Clone)]
pub struct AncestorFilter {
    buckets: [u8; 64],
}

impl Default for AncestorFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl AncestorFilter {
    /// Cria um novo `AncestorFilter` vazio.
    pub fn new() -> Self {
        Self { buckets: [0; 64] }
    }

    /// Limpa todos os contadores do filtro.
    pub fn clear(&mut self) {
        self.buckets.fill(0);
    }

    /// Registra um elemento ancestral no filtro de contagem.
    pub fn push_element(&mut self, el: &ElementData) {
        self.add_atom(&el.tag_name);

        if let Some(ref id) = el.id_attr {
            self.add_atom(id);
        }

        for class in el.classes.as_slice() {
            self.add_atom(class);
        }
    }

    /// Remove um elemento ancestral do filtro de contagem (ao desempilhar na árvore).
    pub fn pop_element(&mut self, el: &ElementData) {
        self.remove_atom(&el.tag_name);

        if let Some(ref id) = el.id_attr {
            self.remove_atom(id);
        }

        for class in el.classes.as_slice() {
            self.remove_atom(class);
        }
    }

    /// Retorna `true` se puder REJEITAR com 100% de certeza que o ancestral não existe na cadeia.
    pub fn fast_reject(&self, compound: &CompoundSelector) -> bool {
        for simple in &compound.simple_selectors {
            match simple {
                SimpleSelector::Tag(tag) if !self.contains_atom(tag) => return true,
                SimpleSelector::Id(id) if !self.contains_atom(id) => return true,
                SimpleSelector::Class(class) if !self.contains_atom(class) => return true,
                _ => {}
            }
        }
        false
    }

    fn hash1(atom: &Atom) -> usize {
        let mut h: u64 = 0x811c9dc5;
        for b in atom.as_str().bytes() {
            h = (h ^ (b as u64)).wrapping_mul(0x01000193);
        }
        (h % 64) as usize
    }

    fn hash2(atom: &Atom) -> usize {
        let mut h: u64 = 5381;
        for b in atom.as_str().bytes() {
            h = ((h << 5).wrapping_add(h)).wrapping_add(b as u64);
        }
        (h % 64) as usize
    }

    fn add_atom(&mut self, atom: &Atom) {
        let h1 = Self::hash1(atom);
        let h2 = Self::hash2(atom);
        self.buckets[h1] = self.buckets[h1].saturating_add(1);
        self.buckets[h2] = self.buckets[h2].saturating_add(1);
    }

    fn remove_atom(&mut self, atom: &Atom) {
        let h1 = Self::hash1(atom);
        let h2 = Self::hash2(atom);
        if self.buckets[h1] < 255 {
            self.buckets[h1] = self.buckets[h1].saturating_sub(1);
        }
        if self.buckets[h2] < 255 {
            self.buckets[h2] = self.buckets[h2].saturating_sub(1);
        }
    }

    fn contains_atom(&self, atom: &Atom) -> bool {
        let h1 = Self::hash1(atom);
        let h2 = Self::hash2(atom);
        self.buckets[h1] > 0 && self.buckets[h2] > 0
    }
}
