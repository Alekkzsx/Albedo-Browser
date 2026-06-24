use super::*;
//! LiveNodeList - Coleções de nós que se atualizam automaticamente
//! 
//! Implementa HTMLCollection e NodeList conforme especificação WHATWG DOM.
//! Diferente de Vec<usize>, estas coleções são "live" - refletem mudanças no DOM
//! sem necessidade de re-query.

use std::cell::RefCell;
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};

/// Trait para tipos de queries suportados por LiveNodeList


/// HTMLCollection - Live collection para elementos (apenas Element nodes)
/// 
/// Similar a LiveNodeList mas retorna apenas elementos e tem métodos adicionais
/// como namedItem() para acesso por name ou id.
pub struct HTMLCollection {
    inner: LiveNodeList<TagNameQuery>,
}

impl HTMLCollection {
    /// TODO: add docs
    pub fn new(tag_name: &str, root_idx: usize) -> Self {
        Self {
            inner: LiveNodeList::new(TagNameQuery(tag_name.to_string()), root_idx),
        }
    }
    
    /// Número de elementos na coleção
    pub fn length(&self, dom: &AceDOM) -> usize {
        self.inner.length(dom)
    }
    
    /// Retorna elemento no índice (ou None)
    pub fn item(&self, dom: &AceDOM, index: usize) -> Option<usize> {
        self.inner.item(dom, index)
    }
    
    /// Retorna elemento por name ou id (primeiro match)
    pub fn named_item(&self, dom: &AceDOM, name: &str) -> Option<usize> {
        let matches = self.inner.to_vec(dom);
        for idx in matches {
            if let Some(node) = dom.get_node(idx) {
                if let AceNodeType::Element(el) = &node.node_type {
                    if el.attributes.get("id").map_or(false, |id| id == name)
                        || el.attributes.get("name").map_or(false, |n| n == name)
                    {
                        return Some(idx);
                    }
                }
            }
        }
        None
    }
    
    /// Marca como dirty (chamar após DOM mutations)
    pub fn mark_dirty(&self) {
        self.inner.mark_dirty();
    }
}
