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

pub struct CustomElementsRegistry {
    definitions: HashMap<String, CustomElementDefinition>,
    upgrading: HashMap<usize, String>, // element_idx -> definition name durante upgrade
}

impl CustomElementsRegistry {
    /// TODO: add docs
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
            upgrading: HashMap::new(),
        }
    }
    
    /// define(name, constructor, options)
    pub fn define(
        &mut self,
        name: String,
        constructor: Arc<dyn Fn() -> usize + Send + Sync>,
        _options: Option<ElementDefinitionOptions>,
    ) -> Result<(), CustomElementError> {
        // Valida o nome (deve conter '-')
        if !name.contains('-') {
            return Err(CustomElementError::InvalidName);
        }
        
        // Verifica se já está definido
        if self.definitions.contains_key(&name) {
            return Err(CustomElementError::AlreadyDefined);
        }
        
        let definition = CustomElementDefinition {
            name: name.clone(),
            constructor,
            local_name: name.clone(), // Por padrão, igual ao name
            is_value: None,
            observed_attributes: Vec::new(), // Será setado via static getter
            form_associated: false,
        };
        
        self.definitions.insert(name, definition);
        Ok(())
    }
    
    /// get(name)
    pub fn get(&self, name: &str) -> Option<&CustomElementDefinition> {
        self.definitions.get(name)
    }
    
    /// whenDefined(name) - retorna uma Promise (simulada aqui)
    pub fn when_defined(&self, name: &str) -> bool {
        self.definitions.contains_key(name)
    }
    
    /// Upgrade um elemento específico
    pub fn upgrade_element(&mut self, dom: &mut AceDOM, element_idx: usize) {
        let Some((tag, is_value)) = dom.get_node(element_idx).and_then(|node| {
            if let AceNodeType::Element(el) = &node.node_type {
                Some((el.tag.to_lowercase(), el.attributes.get("is").cloned()))
            } else {
                None
            }
        }) else {
            return;
        };

        // Verifica se há definição para este tag
        if let Some(definition) = self.definitions.get(&tag).cloned() {
            self.perform_upgrade(dom, element_idx, &definition);
        }

        // Verifica se é um built-in extendido via "is" attribute
        if let Some(is_value) = is_value {
            if let Some(definition) = self.definitions.get(&is_value).cloned() {
                self.perform_upgrade(dom, element_idx, &definition);
            }
        }
    }
    
    /// Upgrade todos os elementos no DOM
    pub fn upgrade_all(&mut self, dom: &mut AceDOM) {
        let element_indices: Vec<usize> = dom.nodes.iter()
            .enumerate()
            .filter_map(|(idx, node)| {
                if matches!(node.node_type, AceNodeType::Element(_)) {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();
        
        for idx in element_indices {
            self.upgrade_element(dom, idx);
        }
    }
    
pub(crate) fn perform_upgrade(
        &mut self,
        dom: &mut AceDOM,
        element_idx: usize,
        definition: &CustomElementDefinition,
    ) {
        // Marca como upgrading
        self.upgrading.insert(element_idx, definition.name.clone());
        
        // Chama o constructor (que deve retornar o índice do elemento)
        let _constructed_idx = (definition.constructor)();
        
        // Se o constructor retornou um elemento diferente, precisamos fazer o swap
        // (implementação simplificada aqui)
        
        // Remove da lista de upgrading
        self.upgrading.remove(&element_idx);
        
        // Chama connectedCallback se o elemento já estiver no DOM
        if self.is_connected(dom, element_idx) {
            self.call_connected_callback(element_idx);
        }
    }
    
pub(crate) fn is_connected(&self, dom: &AceDOM, element_idx: usize) -> bool {
        // Verifica se o elemento tem um caminho até o document root
        let mut current = Some(element_idx);
        while let Some(idx) = current {
            if idx == dom.root {
                return true;
            }
            if let Some(node) = dom.get_node(idx) {
                current = node.parent;
            } else {
                break;
            }
        }
        false
    }
    
pub(crate) fn call_connected_callback(&self, _element_idx: usize) {
        // Em produção, chamaria o método no objeto JavaScript
        // Aqui é apenas um placeholder
    }
    
pub(crate) fn call_disconnected_callback(&self, _element_idx: usize) {
        // Em produção, chamaria o método no objeto JavaScript
    }
    
pub(crate) fn call_adopted_callback(&self, _element_idx: usize) {
        // Chamado quando o elemento é movido para outro documento
    }
    
    /// TODO: add docs
    pub fn call_attribute_changed_callback(
        &self,
        _element_idx: usize,
        _name: &str,
        _old_value: Option<&str>,
        _new_value: Option<&str>,
    ) {
        // Verifica se este atributo é observado
        // Em produção, verificaria no definition.observedAttributes
        // e chamaria o callback se apropriado
    }
}
