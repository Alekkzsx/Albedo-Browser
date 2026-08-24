//! # Pilha de Elementos Abertos (Stack of Open Elements - WHATWG §12.2.4.2)
//!
//! Rastreia a hierarquia de nós atualmente em construção com métodos normativos de escopo.

use crate::tree::Document;
use ace_core::id::NodeId;

/// Pilha de elementos abertos com suporte a verificações normativas de escopo.
#[derive(Debug, Clone, Default)]
pub struct StackOfOpenElements {
    stack: Vec<NodeId>,
}

impl StackOfOpenElements {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(32),
        }
    }

    #[inline]
    pub fn push(&mut self, id: NodeId) {
        self.stack.push(id);
    }

    #[inline]
    pub fn pop(&mut self) -> Option<NodeId> {
        self.stack.pop()
    }

    #[inline]
    pub fn current_node(&self) -> Option<NodeId> {
        self.stack.last().copied()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    #[inline]
    pub fn as_slice(&self) -> &[NodeId] {
        &self.stack
    }

    #[inline]
    pub fn contains(&self, id: NodeId) -> bool {
        self.stack.contains(&id)
    }

    /// Verifica se a pilha contém um elemento com a tag especificada.
    pub fn contains_tag(&self, doc: &Document, tag_name: &str) -> bool {
        self.stack.iter().rev().any(|&id| {
            doc.get_node(id)
                .and_then(|n| n.tag_name())
                .is_some_and(|t| t.eq_ignore_ascii_case(tag_name))
        })
    }

    /// Desempilha nós até encontrar e remover o nó com a tag especificada.
    pub fn pop_until_tag(&mut self, doc: &Document, tag_name: &str) -> Option<NodeId> {
        while let Some(top_id) = self.current_node() {
            let is_match = doc
                .get_node(top_id)
                .and_then(|n| n.tag_name())
                .is_some_and(|t| t.eq_ignore_ascii_case(tag_name));

            self.pop();
            if is_match {
                return Some(top_id);
            }
        }
        None
    }

    /// Desempilha nós até encontrar e remover o nó com o ID informado.
    pub fn pop_until_id(&mut self, target_id: NodeId) -> bool {
        while let Some(top_id) = self.pop() {
            if top_id == target_id {
                return true;
            }
        }
        false
    }

    /// Verifica se um elemento com a tag informada está no escopo padrão do HTML5 (WHATWG §12.2.4.2).
    pub fn has_element_in_scope(&self, doc: &Document, target_tag: &str) -> bool {
        for &node_id in self.stack.iter().rev() {
            if let Some(node) = doc.get_node(node_id) {
                if let Some(tag) = node.tag_name() {
                    if tag.eq_ignore_ascii_case(target_tag) {
                        return true;
                    }
                    // Elementos delimitadores de escopo
                    if matches!(
                        tag.as_str(),
                        "applet"
                            | "caption"
                            | "html"
                            | "table"
                            | "td"
                            | "th"
                            | "marquee"
                            | "object"
                            | "template"
                            | "select"
                    ) {
                        return false;
                    }
                }
            }
        }
        false
    }

    /// Verifica se um elemento está no escopo de tabela (WHATWG §12.2.4.2).
    pub fn has_element_in_table_scope(&self, doc: &Document, target_tag: &str) -> bool {
        for &node_id in self.stack.iter().rev() {
            if let Some(node) = doc.get_node(node_id) {
                if let Some(tag) = node.tag_name() {
                    if tag.eq_ignore_ascii_case(target_tag) {
                        return true;
                    }
                    if matches!(tag.as_str(), "html" | "table" | "template") {
                        return false;
                    }
                }
            }
        }
        false
    }

    /// Verifica se um elemento está no escopo de botão (WHATWG §12.2.4.2).
    pub fn has_element_in_button_scope(&self, doc: &Document, target_tag: &str) -> bool {
        for &node_id in self.stack.iter().rev() {
            if let Some(node) = doc.get_node(node_id) {
                if let Some(tag) = node.tag_name() {
                    if tag.eq_ignore_ascii_case(target_tag) {
                        return true;
                    }
                    if matches!(
                        tag.as_str(),
                        "applet"
                            | "caption"
                            | "html"
                            | "table"
                            | "td"
                            | "th"
                            | "marquee"
                            | "object"
                            | "template"
                            | "button"
                    ) {
                        return false;
                    }
                }
            }
        }
        false
    }

    /// Verifica se um elemento está no escopo de item de lista (WHATWG §12.2.4.2).
    pub fn has_element_in_list_item_scope(&self, doc: &Document, target_tag: &str) -> bool {
        for &node_id in self.stack.iter().rev() {
            if let Some(node) = doc.get_node(node_id) {
                if let Some(tag) = node.tag_name() {
                    if tag.eq_ignore_ascii_case(target_tag) {
                        return true;
                    }
                    if matches!(
                        tag.as_str(),
                        "applet"
                            | "caption"
                            | "html"
                            | "table"
                            | "td"
                            | "th"
                            | "marquee"
                            | "object"
                            | "template"
                            | "ol"
                            | "ul"
                    ) {
                        return false;
                    }
                }
            }
        }
        false
    }
}
