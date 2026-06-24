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


impl AccessibilityTree {
    /// TODO: add docs
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root: None,
        }
    }
    
    /// Constrói a accessibility tree a partir do DOM
    pub fn build(dom: &AceDOM) -> Self {
        let mut tree = Self::new();
        
        // Começa do body ou root
        let start_idx = dom.body.unwrap_or(dom.root);
        
        tree.build_recursive(dom, start_idx, None);
        tree.root = Some(start_idx);
        
        tree
    }
    
pub(crate) fn build_recursive(
        &mut self,
        dom: &AceDOM,
        node_idx: usize,
        parent_idx: Option<usize>,
    ) -> Option<usize> {
        let node = dom.get_node(node_idx)?;
        
        // Verifica visibilidade
        let is_visible = self.is_node_visible(dom, node_idx);
        if !is_visible {
            return None;
        }
        
        // Obtém role implícito
        let role = match &node.node_type {
            AceNodeType::Element(el) => {
                ImplicitRoleMap::get_implicit_role(&el.tag, &el.attributes)
            },
            AceNodeType::Text(_) => return None, // Text nodes não entram na tree
            AceNodeType::Document => AriaRole::Document,
            _ => AriaRole::Generic,
        };
        
        // Pula elementos com role="none" ou "presentation"
        if role == AriaRole::None || role == AriaRole::Presentation {
            // Mas ainda processa children
            for &child_idx in &node.children {
                self.build_recursive(dom, child_idx, parent_idx);
            }
            return None;
        }
        
        // Cria o accessibility node
        let mut acc_node = AccessibilityNode::new(node_idx, role);
        
        // Computa accessible name
        acc_node.name = AccessibleNameComputer::compute_name(dom, node_idx);
        
        // Extrai estados e propriedades dos atributos ARIA
        self.extract_aria_attributes(dom, node_idx, &mut acc_node);
        
        // Determina se é focusable
        acc_node.is_focusable = self.is_focusable(dom, node_idx);
        
        // Processa children
        let mut acc_children = Vec::new();
        for &child_idx in &node.children {
            if let Some(child_acc_idx) = self.build_recursive(dom, child_idx, Some(node_idx)) {
                acc_children.push(child_acc_idx);
            }
        }
        acc_node.children = acc_children;
        acc_node.parent = parent_idx;
        
        // Insere na tree
        self.nodes.insert(node_idx, acc_node);
        
        Some(node_idx)
    }
    
pub(crate) fn is_node_visible(&self, dom: &AceDOM, node_idx: usize) -> bool {
        // Verifica aria-hidden
        if let Some(node) = dom.get_node(node_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                if el.attributes.get("aria-hidden") == Some(&"true".to_string()) {
                    return false;
                }
                
                // Verifica display: none e visibility: hidden (em produção, verificaria computed styles)
                // Aqui é uma verificação simplificada
            }
        }
        
        true
    }
    
pub(crate) fn is_focusable(&self, dom: &AceDOM, node_idx: usize) -> bool {
        if let Some(node) = dom.get_node(node_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                // Elementos naturalmente focusable
                let focusable_tags = ["a", "button", "input", "select", "textarea"];
                if focusable_tags.contains(&el.tag.to_lowercase().as_str()) {
                    // Verifica tabindex
                    if let Some(tabindex) = el.attributes.get("tabindex") {
                        return tabindex.parse::<i32>().unwrap_or(-1) >= 0;
                    }
                    return true;
                }
                
                // tabindex explícito
                if let Some(tabindex) = el.attributes.get("tabindex") {
                    return tabindex.parse::<i32>().unwrap_or(-1) >= 0;
                }
            }
        }
        
        false
    }
}
