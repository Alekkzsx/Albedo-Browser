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


impl AccessibleNameComputer {
    /// Computa o accessible name seguindo AccName 1.2
    /// Prioridade: aria-labelledby > aria-label > title > conteúdo interno
    pub fn compute_name(dom: &AceDOM, node_idx: usize) -> String {
        let node = match dom.get_node(node_idx) {
            Some(n) => n,
            None => return String::new(),
        };
        
        let element = match &node.node_type {
            AceNodeType::Element(el) => el,
            _ => return String::new(),
        };
        
        // 1. Verifica aria-labelledby (IDs referenciados)
        if let Some(labelledby) = element.attributes.get("aria-labelledby") {
            let name = Self::compute_from_labelledby(dom, labelledby);
            if !name.is_empty() {
                return Self::normalize_whitespace(&name);
            }
        }
        
        // 2. Verifica aria-label
        if let Some(label) = element.attributes.get("aria-label") {
            return Self::normalize_whitespace(label);
        }
        
        // 3. Verifica title attribute (fallback)
        if let Some(title) = element.attributes.get("title") {
            return Self::normalize_whitespace(title);
        }
        
        // 4. Computa do conteúdo interno (recursive text collection)
        let content = Self::collect_text_content(dom, node_idx);
        Self::normalize_whitespace(&content)
    }
    
pub(crate) fn compute_from_labelledby(dom: &AceDOM, labelledby: &str) -> String {
        let mut parts = Vec::new();
        
        for id_ref in labelledby.split_whitespace() {
            // Encontra elemento com este ID
            if let Some(ref_idx) = Self::find_element_by_id(dom, id_ref) {
                let text = Self::collect_text_content(dom, ref_idx);
                if !text.is_empty() {
                    parts.push(text);
                }
            }
        }
        
        parts.join(" ")
    }
    
pub(crate) fn find_element_by_id(dom: &AceDOM, id: &str) -> Option<usize> {
        for (idx, node) in dom.nodes.iter().enumerate() {
            if let AceNodeType::Element(el) = &node.node_type {
                if el.attributes.get("id") == Some(&id.to_string()) {
                    return Some(idx);
                }
            }
        }
        None
    }
    
pub(crate) fn collect_text_content(dom: &AceDOM, node_idx: usize) -> String {
        let mut text = String::new();
        let mut stack = vec![node_idx];
        
        while let Some(idx) = stack.pop() {
            if let Some(node) = dom.get_node(idx) {
                match &node.node_type {
                    AceNodeType::Text(content) => {
                        text.push_str(content);
                        text.push(' ');
                    },
                    AceNodeType::Element(_) => {
                        // Adiciona children ao stack
                        stack.extend(&node.children);
                    },
                    _ => {}
                }
            }
        }
        
        text
    }
    
pub(crate) fn normalize_whitespace(s: &str) -> String {
        s.split_whitespace().collect::<Vec<&str>>().join(" ")
    }
}
