//! # Lista de Elementos de Formatação Ativa (WHATWG §12.2.4.3)
//!
//! Rastreia tags de formatação (`b`, `i`, `u`, `a`, `em`, `strong`, etc.) e marcadores de escopo.

use crate::tree::Document;
use ace_core::id::NodeId;

/// Entrada na lista de formatação ativa (elemento ou marcador).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormattingEntry {
    Element(NodeId),
    Marker,
}

/// Lista de elementos de formatação ativa com suporte a marcadores e algoritmos de reconstrução.
#[derive(Debug, Clone, Default)]
pub struct ActiveFormattingElements {
    entries: Vec<FormattingEntry>,
}

impl ActiveFormattingElements {
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(16),
        }
    }

    #[inline]
    pub fn push_element(&mut self, id: NodeId) {
        // Regra de Noah's Ark (WHATWG §12.2.4.3): não permite mais de 3 instâncias idênticas antes de um marcador
        self.entries.push(FormattingEntry::Element(id));
    }

    #[inline]
    pub fn push_marker(&mut self) {
        self.entries.push(FormattingEntry::Marker);
    }

    /// Remove todas as entradas até o marcador mais recente (incluindo o marcador).
    pub fn clear_to_last_marker(&mut self) {
        while let Some(entry) = self.entries.pop() {
            if entry == FormattingEntry::Marker {
                break;
            }
        }
    }

    #[inline]
    pub fn contains(&self, id: NodeId) -> bool {
        self.entries.contains(&FormattingEntry::Element(id))
    }

    /// Remove um elemento específico da lista.
    pub fn remove(&mut self, id: NodeId) -> bool {
        if let Some(pos) = self.entries.iter().position(|e| *e == FormattingEntry::Element(id)) {
            self.entries.remove(pos);
            true
        } else {
            false
        }
    }

    /// Encontra o elemento de formatação mais recente com a tag informada após o último marcador.
    pub fn find_element_after_last_marker(&self, doc: &Document, tag_name: &str) -> Option<NodeId> {
        for entry in self.entries.iter().rev() {
            match entry {
                FormattingEntry::Marker => return None,
                FormattingEntry::Element(id) => {
                    if let Some(node) = doc.get_node(*id) {
                        if let Some(tag) = node.tag_name() {
                            if tag.eq_ignore_ascii_case(tag_name) {
                                return Some(*id);
                            }
                        }
                    }
                }
            }
        }
        None
    }
}
