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


/// Nó na Accessibility Tree
#[derive(Clone, Debug)]
pub struct AccessibilityNode {
    pub node_idx: usize,
    pub role: AriaRole,
    pub name: String,                       // Accessible name (computado)
    pub description: Option<String>,        // Accessible description
    pub states: AriaStates,
    pub properties: AriaProperties,
    pub children: Vec<usize>,               // Índices dos children na accessibility tree
    pub parent: Option<usize>,
    pub is_visible: bool,
    pub is_focusable: bool,
}

impl AccessibilityNode {
    /// TODO: add docs
    pub fn new(node_idx: usize, role: AriaRole) -> Self {
        Self {
            node_idx,
            role,
            name: String::new(),
            description: None,
            states: AriaStates::default(),
            properties: AriaProperties::default(),
            children: Vec::new(),
            parent: None,
            is_visible: true,
            is_focusable: false,
        }
    }
}
