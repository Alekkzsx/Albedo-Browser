//! DomArena - Arena de alocação para AceDOM nodes
//! 
//! Implementa alocação em blocos de 4KB para melhor performance
//! e memória cache-friendly.

use super::*;
use std::alloc::{self, Layout};
use std::cell::Cell;
use std::ptr::NonNull;

/// Tamanho do bloco da arena em bytes (4KB)


/// Estrutura que representa um nó na arena
#[repr(C)]
#[derive(Clone, Debug)]
pub struct ArenaNode {
    pub node_type: crate::ace::engine::dom::AceNodeType,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub prev_sibling: Option<usize>,
    pub next_sibling: Option<usize>,
    pub shadow_root: Option<usize>,
    pub dirty: crate::ace::engine::dom::NodeDirtyFlags,
}

impl ArenaNode {
    /// TODO: add docs
    pub fn new(node_type: crate::ace::engine::dom::AceNodeType) -> Self {
        Self {
            node_type,
            parent: None,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: crate::ace::engine::dom::NodeDirtyFlags::LAYOUT
                | crate::ace::engine::dom::NodeDirtyFlags::STYLE,
        }
    }
    
    /// Retorna o tamanho em bytes deste nó (para cálculo de memória)
    pub fn size_bytes(&self) -> usize {
        let base_size = std::mem::size_of::<Self>();
        let children_size = self.children.capacity() * std::mem::size_of::<usize>();
        
        // Tamanho aproximado do node_type
        let node_type_size = match &self.node_type {
            crate::ace::engine::dom::AceNodeType::Element(el) => {
                el.tag.len() + std::mem::size_of_val(&el.attributes)
            }
            crate::ace::engine::dom::AceNodeType::Text(s)
            | crate::ace::engine::dom::AceNodeType::Comment(s) => s.len(),
            _ => 0,
        };
        
        base_size + children_size + node_type_size
    }
}
