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
    
pub(crate) fn extract_aria_attributes(&self, dom: &AceDOM, node_idx: usize, acc_node: &mut AccessibilityNode) {
        if let Some(node) = dom.get_node(node_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                // States
                if let Some(checked) = el.attributes.get("aria-checked") {
                    acc_node.states.checked = Some(checked == "true");
                }
                if let Some(selected) = el.attributes.get("aria-selected") {
                    acc_node.states.selected = Some(selected == "true");
                }
                if let Some(expanded) = el.attributes.get("aria-expanded") {
                    acc_node.states.expanded = Some(expanded == "true");
                }
                if let Some(hidden) = el.attributes.get("aria-hidden") {
                    acc_node.states.hidden = hidden == "true";
                }
                if let Some(disabled) = el.attributes.get("aria-disabled") {
                    acc_node.states.disabled = disabled == "true";
                }
                
                // Properties
                if let Some(label) = el.attributes.get("aria-label") {
                    acc_node.properties.label = Some(label.clone());
                }
                if let Some(labelledby) = el.attributes.get("aria-labelledby") {
                    acc_node.properties.labelledby = labelledby.split_whitespace()
                        .map(|s| s.to_string()).collect();
                }
                if let Some(describedby) = el.attributes.get("aria-describedby") {
                    acc_node.properties.describedby = describedby.split_whitespace()
                        .map(|s| s.to_string()).collect();
                }
                if let Some(controls) = el.attributes.get("aria-controls") {
                    acc_node.properties.controls = controls.split_whitespace()
                        .map(|s| s.to_string()).collect();
                }
                if let Some(level) = el.attributes.get("aria-level") {
                    acc_node.properties.level = level.parse().ok();
                }
                if let Some(valuenow) = el.attributes.get("aria-valuenow") {
                    acc_node.properties.valuenow = valuenow.parse().ok();
                }
            }
        }
    }
    
    /// Retorna os children acessíveis de um nó
    pub fn get_accessible_children(&self, node_idx: usize) -> Vec<&AccessibilityNode> {
        if let Some(node) = self.nodes.get(&node_idx) {
            node.children.iter()
                .filter_map(|&idx| self.nodes.get(&idx))
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Retorna o parent acessível de um nó
    pub fn get_accessible_parent(&self, node_idx: usize) -> Option<&AccessibilityNode> {
        if let Some(node) = self.nodes.get(&node_idx) {
            node.parent.and_then(|p| self.nodes.get(&p))
        } else {
            None
        }
    }
    
    /// Traverse pre-order
    pub fn traverse_pre_order<F>(&self, callback: &mut F)
    where
        F: FnMut(&AccessibilityNode),
    {
        if let Some(root_idx) = self.root {
            self.traverse_pre_order_recursive(root_idx, callback);
        }
    }
    
pub(crate) fn traverse_pre_order_recursive<F>(&self, node_idx: usize, callback: &mut F)
    where
        F: FnMut(&AccessibilityNode),
    {
        if let Some(node) = self.nodes.get(&node_idx) {
            callback(node);
            for &child_idx in &node.children {
                self.traverse_pre_order_recursive(child_idx, callback);
            }
        }
    }
}
