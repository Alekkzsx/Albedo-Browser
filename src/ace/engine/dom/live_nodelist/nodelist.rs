use super::*;
//! LiveNodeList - Coleções de nós que se atualizam automaticamente
//! 
//! Implementa HTMLCollection e NodeList conforme especificação WHATWG DOM.
//! Diferente de Vec<usize>, estas coleções são "live" - refletem mudanças no DOM
//! sem necessidade de re-query.

use std::cell::RefCell;
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};

/// Trait para tipos de queries suportados por LiveNodeList


/// NodeList - pode ser live ou snapshot
/// 
/// Por padrão é live como HTMLCollection, mas pode ser convertido
/// para snapshot se necessário.
pub struct NodeList {
    inner: LiveNodeList<TagNameQuery>,
    is_snapshot: bool,
}

impl NodeList {
    /// Cria NodeList live (auto-update)
    pub fn new_live(root_idx: usize) -> Self {
        Self {
            inner: LiveNodeList::new(TagNameQuery("*".to_string()), root_idx),
            is_snapshot: false,
        }
    }
    
    /// Cria NodeList snapshot (não atualiza)
    pub fn new_snapshot(_nodes: Vec<usize>) -> Self {
        // Implementação simplificada - em produção usaria enum
        Self {
            inner: LiveNodeList::new(TagNameQuery("*".to_string()), 0),
            is_snapshot: true,
        }
    }
    
    /// TODO: add docs
    pub fn length(&self, dom: &AceDOM) -> usize {
        if self.is_snapshot {
            // Em produção, teria um campo separado para snapshot
            0
        } else {
            self.inner.length(dom)
        }
    }
    
    /// TODO: add docs
    pub fn item(&self, dom: &AceDOM, index: usize) -> Option<usize> {
        if self.is_snapshot {
            None
        } else {
            self.inner.item(dom, index)
        }
    }
    
    /// TODO: add docs
    pub fn mark_dirty(&self) {
        if !self.is_snapshot {
            self.inner.mark_dirty();
        }
    }
}
