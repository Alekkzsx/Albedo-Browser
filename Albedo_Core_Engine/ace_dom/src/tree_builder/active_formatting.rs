//! # Lista de Elementos de Formatação Ativa (WHATWG §12.2.4.3)
//!
//! Rastreia tags de formatação (`b`, `i`, `u`, `a`, `em`, `strong`, etc.) e marcadores de escopo,
//! aplicando a Cláusula "Arca de Noé" (*Noah's Ark Clause*) para prevenir estouro de memória e DoS.

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

    /// Adiciona um novo elemento de formatação aplicando a Cláusula Arca de Noé (*Noah's Ark Clause*).
    ///
    /// Se já existirem 3 elementos idênticos (mesma tag, namespace e atributos) após o último marcador,
    /// a entrada mais antiga é removida para prevenir consumo quadrático de memória.
    pub fn push_element(&mut self, doc: &Document, id: NodeId) {
        if let Some(target_node) = doc.get_node(id) {
            if let Some(target_el) = target_node.as_element() {
                let mut match_positions = Vec::new();

                // Varre do topo até o último marcador
                for (idx, entry) in self.entries.iter().enumerate().rev() {
                    match entry {
                        FormattingEntry::Marker => break,
                        FormattingEntry::Element(existing_id) => {
                            if let Some(existing_node) = doc.get_node(*existing_id) {
                                if let Some(existing_el) = existing_node.as_element() {
                                    if existing_el.tag_name == target_el.tag_name
                                        && existing_el.namespace == target_el.namespace
                                        && existing_el.attributes.as_slice() == target_el.attributes.as_slice()
                                    {
                                        match_positions.push(idx);
                                    }
                                }
                            }
                        }
                    }
                }

                // Se houver 3 ou mais instâncias idênticas, remove a mais antiga (menor índice)
                if match_positions.len() >= 3 {
                    if let Some(&earliest_idx) = match_positions.last() {
                        self.entries.remove(earliest_idx);
                    }
                }
            }
        }

        self.entries.push(FormattingEntry::Element(id));
    }

    /// Insere uma entrada simples sem verificação (para testes e reconstrução).
    #[inline]
    pub fn push_raw(&mut self, id: NodeId) {
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

    /// Retorna o número de entradas ativas.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Retorna `true` se a lista de formatação estiver vazia.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
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

    /// Retorna as entradas como fatia.
    pub fn as_slice(&self) -> &[FormattingEntry] {
        &self.entries
    }
}
