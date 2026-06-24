use super::*;
// Custom Elements v1 Implementation - WHATWG Spec
//
// Este módulo implementa:
// - customElements.define()
// - Lifecycle callbacks (connected, disconnected, adopted, attributeChanged)
// - Upgrade algorithm
// - Built-in element extension

use std::collections::HashMap;
use std::sync::Arc;
use crate::ace::engine::dom::{AceDOM, AceNodeType, AceNode, AceElement, NodeDirtyFlags};

/// Registry global de Custom Elements


#[cfg(test)]
mod tests {
    use crate::ace::engine::dom::{AceElement, AceNode, NodeDirtyFlags};
    use super::*;
    use crate::ace::engine::dom::AceDOM;
    
    #[test]
pub(crate) fn test_define_valid_name() {
        let mut registry = CustomElementsRegistry::new();
        
        let constructor = Arc::new(|| 0usize) as Arc<dyn Fn() -> usize + Send + Sync>;
        let result = registry.define("my-element".to_string(), constructor, None);
        
        assert!(result.is_ok());
    }
    
    #[test]
pub(crate) fn test_define_invalid_name_no_dash() {
        let mut registry = CustomElementsRegistry::new();
        
        let constructor = Arc::new(|| 0usize) as Arc<dyn Fn() -> usize + Send + Sync>;
        let result = registry.define("myelement".to_string(), constructor, None);
        
        assert_eq!(result, Err(CustomElementError::InvalidName));
    }
    
    #[test]
pub(crate) fn test_define_already_defined() {
        let mut registry = CustomElementsRegistry::new();
        
        let constructor = Arc::new(|| 0usize) as Arc<dyn Fn() -> usize + Send + Sync>;
        let first = registry.define("my-element".to_string(), constructor.clone(), None);
        let second = registry.define("my-element".to_string(), constructor, None);
        
        assert!(first.is_ok());
        assert_eq!(second, Err(CustomElementError::AlreadyDefined));
    }
    
    #[test]
pub(crate) fn test_get_definition() {
        let mut registry = CustomElementsRegistry::new();
        
        let constructor = Arc::new(|| 0usize) as Arc<dyn Fn() -> usize + Send + Sync>;
        registry.define("my-element".to_string(), constructor, None).expect("Albedo Engine: internal invariant violated");
        
        let definition = registry.get("my-element");
        assert!(definition.is_some());
        
        let not_found = registry.get("non-existent");
        assert!(not_found.is_none());
    }
    
    #[test]
pub(crate) fn test_when_defined() {
        let mut registry = CustomElementsRegistry::new();
        
        let constructor = Arc::new(|| 0usize) as Arc<dyn Fn() -> usize + Send + Sync>;
        
        assert!(!registry.when_defined("my-element"));
        
        registry.define("my-element".to_string(), constructor, None).expect("Albedo Engine: internal invariant violated");
        
        assert!(registry.when_defined("my-element"));
    }
    
    #[test]
pub(crate) fn test_upgrade_element() {
        let mut dom = AceDOM::from_html("<my-element></my-element>");
        let mut registry = CustomElementsRegistry::new();
        
        let element_idx = dom.body.map(|b| {
            if let Some(node) = dom.get_node(b) {
                node.children.first().copied().unwrap_or(b)
            } else {
                b
            }
        }).unwrap_or(1);
        
        let constructor = Arc::new(move || element_idx) as Arc<dyn Fn() -> usize + Send + Sync>;
        registry.define("my-element".to_string(), constructor, None).expect("Albedo Engine: internal invariant violated");
        
        registry.upgrade_element(&mut dom, element_idx);
        
        // O upgrade foi tentado (em produção, chamaria o constructor)
    }
    
    #[test]
pub(crate) fn test_is_connected_true() {
        let dom = AceDOM::from_html("<div><span></span></div>");
        let registry = CustomElementsRegistry::new();
        
        // span deve estar conectado
        let span_idx = 2;
        assert!(registry.is_connected(&dom, span_idx));
    }
    
    #[test]
pub(crate) fn test_is_connected_false() {
        let mut dom = AceDOM::from_html("<div><span></span></div>");
        let registry = CustomElementsRegistry::new();
        
        // Cria um nó desconectado
        let disconnected_idx = dom.nodes.len();
        dom.nodes.push(AceNode {
            node_type: AceNodeType::Element(AceElement {
                tag: "div".to_string(),
                namespace: crate::ace::html::Namespace::Html,
                attributes: HashMap::new(),
            }),
            parent: None,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: NodeDirtyFlags::NONE,
        });
        
        assert!(!registry.is_connected(&dom, disconnected_idx));
    }
    
    #[test]
pub(crate) fn test_set_attribute_with_callback() {
        let mut dom = AceDOM::from_html("<div id=\"test\"></div>");
        let div_idx = dom.body.map(|b| {
            if let Some(node) = dom.get_node(b) {
                node.children.first().copied().unwrap_or(b)
            } else {
                b
            }
        }).unwrap_or(1);
        
        dom.set_attribute_with_callback(div_idx, "class".to_string(), "active".to_string());
        
        // Verifica que o atributo foi setado
        if let Some(node) = dom.get_node(div_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                assert_eq!(el.attributes.get("class"), Some(&"active".to_string()));
            }
        }
    }
    
    #[test]
pub(crate) fn test_remove_attribute_with_callback() {
        let mut dom = AceDOM::from_html("<div class=\"test\"></div>");
        let div_idx = dom.body.map(|b| {
            if let Some(node) = dom.get_node(b) {
                node.children.first().copied().unwrap_or(b)
            } else {
                b
            }
        }).unwrap_or(1);
        
        dom.remove_attribute_with_callback(div_idx, "class".to_string());
        
        // Verifica que o atributo foi removido
        if let Some(node) = dom.get_node(div_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                assert!(!el.attributes.contains_key("class"));
            }
        }
    }
}
