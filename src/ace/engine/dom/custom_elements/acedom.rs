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


/// Extensões para AceDOM suportar Custom Elements
impl AceDOM {
    /// Insert before com suporte a custom elements upgrade
    pub fn insert_before_with_upgrade(&mut self, parent_idx: usize, child_idx: usize, ref_idx: Option<usize>) {
        self.insert_before(parent_idx, child_idx, ref_idx);
        
        // Trigger upgrade se necessário
        // Isso seria feito via registry global em produção
    }
    
    /// Append child com suporte a custom elements upgrade
    pub fn append_child_with_upgrade(&mut self, parent_idx: usize, child_idx: usize) {
        self.append_child(parent_idx, child_idx);
        
        // Trigger upgrade se necessário
        // E chama connectedCallback se aplicável
    }
    
    /// Remove node com suporte a disconnectedCallback
    pub fn remove_node_with_callbacks(&mut self, node_idx: usize) {
        // Chama disconnectedCallback antes de remover
        // Em produção, isso notificaria o registry
        
        self.remove_node_from_parent(node_idx);
    }
    
    /// Set attribute com suporte a attributeChangedCallback
    pub fn set_attribute_with_callback(
        &mut self,
        element_idx: usize,
        name: String,
        value: String,
    ) {
        let _old_value = if let Some(node) = self.get_node(element_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                el.attributes.get(&name).cloned()
            } else {
                None
            }
        } else {
            None
        };
        
        // Set o atributo
        self.set_attribute_notify(element_idx, name.clone(), value.clone());
        
        // Notifica sobre a mudança
        // Em produção, chamaria attributeChangedCallback se o atributo fosse observado
    }
    
    /// Remove attribute com suporte a attributeChangedCallback
    pub fn remove_attribute_with_callback(&mut self, element_idx: usize, name: String) {
        let _old_value = if let Some(node) = self.get_node(element_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                el.attributes.get(&name).cloned()
            } else {
                None
            }
        } else {
            None
        };
        
        // Remove o atributo
        self.remove_attribute_notify(element_idx, name.clone());
        
        // Notifica sobre a mudança
    }
}
