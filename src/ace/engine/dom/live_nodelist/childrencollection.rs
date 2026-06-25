//! LiveNodeList - Coleções de nós que se atualizam automaticamente
//! 
//! Implementa HTMLCollection e NodeList conforme especificação WHATWG DOM.
//! Diferente de Vec<usize>, estas coleções são "live" - refletem mudanças no DOM
//! sem necessidade de re-query.

use super::*;
use std::cell::RefCell;
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};

/// Trait para tipos de queries suportados por LiveNodeList


/// ChildrenCollection - HTMLCollection live dos children de um elemento
pub struct ChildrenCollection {
    parent_idx: usize,
    cache: RefCell<Option<Vec<usize>>>,
    dirty: RefCell<bool>,
}

impl ChildrenCollection {
    /// TODO: add docs
    pub fn new(parent_idx: usize) -> Self {
        Self {
            parent_idx,
            cache: RefCell::new(None),
            dirty: RefCell::new(true),
        }
    }
    
pub(crate) fn compute_children(&self, dom: &AceDOM) -> Vec<usize> {
        if let Some(parent) = dom.get_node(self.parent_idx) {
            parent.children.iter().copied().filter(|&idx| {
                if let Some(node) = dom.get_node(idx) {
                    matches!(node.node_type, AceNodeType::Element(_))
                } else {
                    false
                }
            }).collect()
        } else {
            Vec::new()
        }
    }
    
    /// TODO: add docs
    pub fn length(&self, dom: &AceDOM) -> usize {
        if *self.dirty.borrow() {
            self.compute_children(dom).len()
        } else {
            self.cache.borrow().as_ref().map_or(0, |v| v.len())
        }
    }
    
    /// TODO: add docs
    pub fn item(&self, dom: &AceDOM, index: usize) -> Option<usize> {
        let mut cache_ref = self.cache.borrow_mut();
        
        if *self.dirty.borrow() || cache_ref.is_none() {
            *cache_ref = Some(self.compute_children(dom));
            *self.dirty.borrow_mut() = false;
        }
        
        cache_ref.as_ref().and_then(|children| children.get(index).copied())
    }
    
    /// TODO: add docs
    pub fn mark_dirty(&self) {
        *self.dirty.borrow_mut() = true;
        *self.cache.borrow_mut() = None;
    }
}
