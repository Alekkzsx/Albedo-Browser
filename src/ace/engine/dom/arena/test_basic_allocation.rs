//! DomArena - Arena de alocação para AceDOM nodes
//! 
//! Implementa alocação em blocos de 4KB para melhor performance
//! e memória cache-friendly.

use super::*;
use std::alloc::{self, Layout};
use std::cell::Cell;
use std::ptr::NonNull;

/// Tamanho do bloco da arena em bytes (4KB)


#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::engine::dom::{AceElement, AceNodeType, NodeDirtyFlags};
    
    #[test]
pub(crate) fn test_basic_allocation() {
        let mut arena = DomArena::new(10);
        
        let node = ArenaNode::new(AceNodeType::Document);
        let node_index = arena.alloc(node);
        
        assert!(arena.get(idx).is_some());
        assert_eq!(arena.len(), 1);
    }
    
    #[test]
pub(crate) fn test_multiple_allocations() {
        let mut arena = DomArena::new(5);
        
        let mut indices = Vec::new();
        for i in 0..10 {
            let node = ArenaNode::new(AceNodeType::Text(format!("text{}", i).into()));
            let node_index = arena.alloc(node);
            indices.push(idx);
        }
        
        assert_eq!(arena.len(), 10);
        
        // Verifica se todos os nodes estão acessíveis
        for (i, &idx) in indices.iter().enumerate() {
            assert!(arena.get(idx).is_some());
        }
    }
    
    #[test]
pub(crate) fn test_memory_stats() {
        let mut arena = DomArena::new(100);
        
        for i in 0..50 {
            let node = ArenaNode::new(AceNodeType::Text(format!("text{}", i).into()));
            arena.alloc(node);
        }
        
        let stats = arena.memory_stats();
        assert!(stats.used_bytes > 0);
        assert!(stats.total_bytes >= stats.used_bytes);
        assert!(stats.efficiency() > 0.0);
    }
    
    #[test]
pub(crate) fn test_pooling() {
        let mut arena = DomArena::new(10);
        
        // Aloca alguns divs
        let div1 = ArenaNode::new(AceNodeType::Element(AceElement {
            tag: "div".to_string(),
            namespace: crate::ace::html::Namespace::Html,
            attributes: std::collections::HashMap::new(),
        }));
        let idx1 = arena.alloc_pooled(div1);
        
        // Libera
        arena.dealloc(idx1);
        
        // Aloca outro div - deve reutilizar
        let div2 = ArenaNode::new(AceNodeType::Element(AceElement {
            tag: "div".to_string(),
            namespace: crate::ace::html::Namespace::Html,
            attributes: std::collections::HashMap::new(),
        }));
        let idx2 = arena.alloc_pooled(div2);
        
        // Deve ter reutilizado o mesmo índice
        assert_eq!(idx1, idx2);
    }
}
