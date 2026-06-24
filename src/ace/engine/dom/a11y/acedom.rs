use super::*;
//! Accessibility Tree (ARIA 1.2) Implementation - W3C WAI-ARIA Spec
//! 
//! Este módulo implementa:
//! - Mapeamento implícito de roles HTML → ARIA
//! - Accessible Name Computation (AccName 1.2)
//! - States & Properties ARIA
//! - Relations (aria-controls, aria-owns, etc.)
//! - Tree traversal para screen readers

use std::collections::HashMap;
use crate::ace::engine::dom::{AceDOM, AceNodeType};

/// Role ARIA de um elemento


/// Extensões para AceDOM
impl AceDOM {
    /// Constrói a accessibility tree
    pub fn build_accessibility_tree(&self) -> AccessibilityTree {
        AccessibilityTree::build(self)
    }
    
    /// Verifica se um nó é visível para acessibilidade
    pub fn is_accessible(&self, node_idx: usize) -> bool {
        // Verificações básicas de visibilidade
        if let Some(node) = self.get_node(node_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                // aria-hidden
                if el.attributes.get("aria-hidden") == Some(&"true".to_string()) {
                    return false;
                }
            }
        }
        true
    }
}
