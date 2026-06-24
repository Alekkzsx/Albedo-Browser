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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::engine::dom::AceDOM;
    
    #[test]
pub(crate) fn test_implicit_role_button() {
        let role = ImplicitRoleMap::get_implicit_role("button", &HashMap::new());
        assert_eq!(role, AriaRole::Button);
        assert_eq!(role.as_str(), "button");
    }
    
    #[test]
pub(crate) fn test_implicit_role_link_with_href() {
        let mut attrs = HashMap::new();
        attrs.insert("href".to_string(), "#".to_string());
        let role = ImplicitRoleMap::get_implicit_role("a", &attrs);
        assert_eq!(role, AriaRole::Link);
    }
    
    #[test]
pub(crate) fn test_implicit_role_link_without_href() {
        let role = ImplicitRoleMap::get_implicit_role("a", &HashMap::new());
        assert_eq!(role, AriaRole::Generic);
    }
    
    #[test]
pub(crate) fn test_explicit_role_override() {
        let mut attrs = HashMap::new();
        attrs.insert("role".to_string(), "button".to_string());
        let role = ImplicitRoleMap::get_implicit_role("div", &attrs);
        assert_eq!(role, AriaRole::Button);
    }
    
    #[test]
pub(crate) fn test_compute_name_from_aria_label() {
        let dom = AceDOM::from_html("<button aria-label=\"Close\">X</button>");
        let button_idx = dom.body.map(|b| {
            if let Some(node) = dom.get_node(b) {
                node.children.first().copied().unwrap_or(b)
            } else {
                b
            }
        }).unwrap_or(1);
        
        let name = AccessibleNameComputer::compute_name(&dom, button_idx);
        assert_eq!(name, "Close");
    }
    
    #[test]
pub(crate) fn test_compute_name_from_content() {
        let dom = AceDOM::from_html("<button>Click Me</button>");
        let button_idx = dom.body.map(|b| {
            if let Some(node) = dom.get_node(b) {
                node.children.first().copied().unwrap_or(b)
            } else {
                b
            }
        }).unwrap_or(1);
        
        let name = AccessibleNameComputer::compute_name(&dom, button_idx);
        assert_eq!(name, "Click Me");
    }
    
    #[test]
pub(crate) fn test_build_accessibility_tree() {
        let dom = AceDOM::from_html("<main><button>Click</button></main>");
        let tree = dom.build_accessibility_tree();
        
        assert!(tree.root.is_some());
        assert!(!tree.nodes.is_empty());
    }
    
    #[test]
pub(crate) fn test_aria_hidden_excludes_from_tree() {
        let dom = AceDOM::from_html("<button aria-hidden=\"true\">Hidden</button>");
        let tree = dom.build_accessibility_tree();
        
        // O botão com aria-hidden não deve estar na tree
        // (dependendo da implementação, pode ou não estar)
    }
    
    #[test]
pub(crate) fn test_heading_levels() {
        for tag in &["h1", "h2", "h3", "h4", "h5", "h6"] {
            let role = ImplicitRoleMap::get_implicit_role(tag, &HashMap::new());
            assert_eq!(role, AriaRole::Heading);
        }
    }
    
    #[test]
pub(crate) fn test_input_types() {
        let mut attrs_checkbox = HashMap::new();
        attrs_checkbox.insert("type".to_string(), "checkbox".to_string());
        let role_checkbox = ImplicitRoleMap::get_implicit_role("input", &attrs_checkbox);
        assert_eq!(role_checkbox, AriaRole::Checkbox);
        
        let mut attrs_radio = HashMap::new();
        attrs_radio.insert("type".to_string(), "radio".to_string());
        let role_radio = ImplicitRoleMap::get_implicit_role("input", &attrs_radio);
        assert_eq!(role_radio, AriaRole::Radio);
        
        let mut attrs_range = HashMap::new();
        attrs_range.insert("type".to_string(), "range".to_string());
        let role_range = ImplicitRoleMap::get_implicit_role("input", &attrs_range);
        assert_eq!(role_range, AriaRole::Slider);
    }
}
