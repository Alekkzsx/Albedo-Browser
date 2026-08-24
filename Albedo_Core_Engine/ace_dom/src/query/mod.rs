//! # Motor de Consultas e Seletores DOM (Query Engine)
//!
//! Fornece APIs normativas `getElementById`, `getElementsByTagName`, `getElementsByClassName`,
//! `querySelector` e `querySelectorAll` com suporte a combinadores hierárquicos.

pub mod bloom;
pub mod index;
pub mod selector;

pub use bloom::AncestorFilter;
pub use index::ElementIndex;
pub use selector::{Combinator, ComplexSelector, CompoundSelector, PseudoClass, SimpleSelector};

use crate::tree::Document;
use ace_core::id::NodeId;

impl Document {
    /// Encontra o primeiro elemento com o ID especificado.
    pub fn get_element_by_id(&self, id: &str) -> Option<NodeId> {
        let target_atom = ace_core::intern::Atom::new(id);
        for (node_id, node) in self.descendants(self.root()) {
            if let Some(el) = node.as_element() {
                if el.id_attr.as_ref() == Some(&target_atom) {
                    return Some(node_id);
                }
            }
        }
        None
    }

    /// Encontra todos os elementos com a tag informada (ex: `"div"` ou `"*"`).
    pub fn get_elements_by_tag_name(&self, tag_name: &str) -> Vec<NodeId> {
        let is_universal = tag_name == "*";
        let target_atom = ace_core::intern::Atom::new(&tag_name.to_ascii_lowercase());

        let mut results = Vec::new();
        for (node_id, node) in self.descendants(self.root()) {
            if let Some(el) = node.as_element() {
                if is_universal || el.tag_name == target_atom {
                    results.push(node_id);
                }
            }
        }
        results
    }

    /// Encontra todos os elementos que contêm a classe informada.
    pub fn get_elements_by_class_name(&self, class_name: &str) -> Vec<NodeId> {
        let mut results = Vec::new();
        for (node_id, node) in self.descendants(self.root()) {
            if let Some(el) = node.as_element() {
                if el.has_class(class_name) {
                    results.push(node_id);
                }
            }
        }
        results
    }

    /// Retorna o primeiro elemento que satisfaz a regra de seletor CSS informada (com combinadores).
    pub fn query_selector(&self, selector_str: &str) -> Option<NodeId> {
        let selector = ComplexSelector::parse(selector_str)?;
        self.descendants(self.root())
            .map(|(node_id, _)| node_id)
            .find(|&node_id| selector.matches(self, node_id))
    }

    /// Retorna todos os elementos que satisfazem a regra de seletor CSS informada (com combinadores).
    pub fn query_selector_all(&self, selector_str: &str) -> Vec<NodeId> {
        let selector = match ComplexSelector::parse(selector_str) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut results = Vec::new();
        for (node_id, _) in self.descendants(self.root()) {
            if selector.matches(self, node_id) {
                results.push(node_id);
            }
        }
        results
    }
}
