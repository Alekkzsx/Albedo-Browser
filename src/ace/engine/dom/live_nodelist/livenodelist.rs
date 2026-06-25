//! LiveNodeList - Coleções de nós que se atualizam automaticamente
//! 
//! Implementa HTMLCollection e NodeList conforme especificação WHATWG DOM.
//! Diferente de Vec<usize>, estas coleções são "live" - refletem mudanças no DOM
//! sem necessidade de re-query.

use super::*;
use std::cell::RefCell;
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};

/// Trait para tipos de queries suportados por LiveNodeList


/// LiveNodeList - coleção que se atualiza automaticamente
/// 
/// Diferente de NodeList estático do DOM tradicional, esta implementação
/// usa lazy evaluation: os nós são computados sob demanda quando o índice
/// é acessado, garantindo que sempre reflitam o estado atual do DOM.
pub struct LiveNodeList<Q: NodeQuery> {
    query: Q,
    root_idx: usize,
    /// Cache opcional para performance (invalidado em mutations)
    cache: RefCell<Option<Vec<usize>>>,
    /// Flag para forçar recomputação
    dirty: RefCell<bool>,
}

impl<Q: NodeQuery + Clone> LiveNodeList<Q> {
    /// TODO: add docs
    pub fn new(query: Q, root_idx: usize) -> Self {
        Self {
            query,
            root_idx,
            cache: RefCell::new(None),
            dirty: RefCell::new(true),
        }
    }
    
    /// Marca a lista como suja (precisa recomputar)
    pub fn mark_dirty(&self) {
        *self.dirty.borrow_mut() = true;
        *self.cache.borrow_mut() = None;
    }
    
    /// Computa todos os nós matching (DFS traversal)
pub(crate) fn compute_matches(&self, dom: &AceDOM) -> Vec<usize> {
        let mut matches = Vec::new();
        self.collect_matches(self.root_idx, dom, &mut matches);
        matches
    }
    
    /// Coleta recursivamente nós que matcham a query
pub(crate) fn collect_matches(&self, node_idx: usize, dom: &AceDOM, matches: &mut Vec<usize>) {
        if let Some(node) = dom.get_node(node_idx) {
            if self.query.matches(node, dom) {
                matches.push(node_idx);
            }
            
            // Recurse into children
            for &child_idx in &node.children {
                self.collect_matches(child_idx, dom, matches);
            }
        }
    }
    
    /// Retorna o número de nós na coleção
    pub fn length(&self, dom: &AceDOM) -> usize {
        if *self.dirty.borrow() {
            self.compute_matches(dom).len()
        } else {
            self.cache.borrow().as_ref().map_or(0, |v| v.len())
        }
    }
    
    /// Retorna o nó no índice especificado (ou None)
    pub fn item(&self, dom: &AceDOM, index: usize) -> Option<usize> {
        let mut cache_ref = self.cache.borrow_mut();
        
        if *self.dirty.borrow() || cache_ref.is_none() {
            *cache_ref = Some(self.compute_matches(dom));
            *self.dirty.borrow_mut() = false;
        }
        
        cache_ref.as_ref().and_then(|matches| matches.get(index).copied())
    }
    
    /// Retorna todos os nós como Vec (snapshot)
    pub fn to_vec(&self, dom: &AceDOM) -> Vec<usize> {
        let mut cache_ref = self.cache.borrow_mut();
        
        if *self.dirty.borrow() || cache_ref.is_none() {
            *cache_ref = Some(self.compute_matches(dom));
            *self.dirty.borrow_mut() = false;
        }
        
        cache_ref.as_ref().cloned().unwrap_or_default()
    }
    
    /// Itera sobre todos os nós matching
    pub fn for_each<F>(&self, dom: &AceDOM, mut f: F)
    where
        F: FnMut(usize, &AceNode),
    {
        let matches = self.to_vec(dom);
        for idx in matches {
            if let Some(node) = dom.get_node(idx) {
                f(idx, node);
            }
        }
    }
}
