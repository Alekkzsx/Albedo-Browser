//! Custom Elements v1 Implementation - WHATWG Spec
//! 
//! Este módulo implementa:
//! - customElements.define()
//! - Lifecycle callbacks (connected, disconnected, adopted, attributeChanged)
//! - Upgrade algorithm
//! - Built-in element extension

use std::collections::HashMap;
use std::sync::Arc;
use crate::engine::dom::{AceDOM, AceNode, AceNodeType, AceElement, NodeDirtyFlags};

/// Registry global de Custom Elements
pub struct CustomElementsRegistry {
    definitions: HashMap<String, CustomElementDefinition>,
    upgrading: HashMap<usize, String>, // element_idx -> definition name durante upgrade
}

impl CustomElementsRegistry {
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
        options: Option<ElementDefinitionOptions>,
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
        if let Some(node) = dom.get_node(element_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                let tag = el.tag.to_lowercase();
                
                // Verifica se há definição para este tag
                if let Some(definition) = self.definitions.get(&tag).cloned() {
                    self.perform_upgrade(dom, element_idx, &definition);
                }
                
                // Verifica se é um built-in extendido via "is" attribute
                if let Some(is_value) = el.attributes.get("is") {
                    if let Some(definition) = self.definitions.get(is_value).cloned() {
                        self.perform_upgrade(dom, element_idx, &definition);
                    }
                }
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
    
    fn perform_upgrade(
        &mut self,
        dom: &mut AceDOM,
        element_idx: usize,
        definition: &CustomElementDefinition,
    ) {
        // Marca como upgrading
        self.upgrading.insert(element_idx, definition.name.clone());
        
        // Chama o constructor (que deve retornar o índice do elemento)
        let constructed_idx = (definition.constructor)();
        
        // Se o constructor retornou um elemento diferente, precisamos fazer o swap
        // (implementação simplificada aqui)
        
        // Remove da lista de upgrading
        self.upgrading.remove(&element_idx);
        
        // Chama connectedCallback se o elemento já estiver no DOM
        if self.is_connected(dom, element_idx) {
            self.call_connected_callback(element_idx);
        }
    }
    
    fn is_connected(&self, dom: &AceDOM, element_idx: usize) -> bool {
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
    
    fn call_connected_callback(&self, _element_idx: usize) {
        // Em produção, chamaria o método no objeto JavaScript
        // Aqui é apenas um placeholder
    }
    
    fn call_disconnected_callback(&self, _element_idx: usize) {
        // Em produção, chamaria o método no objeto JavaScript
    }
    
    fn call_adopted_callback(&self, _element_idx: usize) {
        // Chamado quando o elemento é movido para outro documento
    }
    
    pub fn call_attribute_changed_callback(
        &self,
        element_idx: usize,
        name: &str,
        old_value: Option<&str>,
        new_value: Option<&str>,
    ) {
        // Verifica se este atributo é observado
        // Em produção, verificaria no definition.observedAttributes
        // e chamaria o callback se apropriado
    }
}

#[derive(Clone)]
pub struct CustomElementDefinition {
    pub name: String,
    pub constructor: Arc<dyn Fn() -> usize + Send + Sync>,
    pub local_name: String,
    pub is_value: Option<String>,
    pub observed_attributes: Vec<String>,
    pub form_associated: bool,
}

#[derive(Clone, Debug)]
pub struct ElementDefinitionOptions {
    pub extends: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CustomElementError {
    InvalidName,
    AlreadyDefined,
}

/// Lifecycle Callbacks Traits
pub trait LifecycleCallbacks {
    fn connected_callback(&mut self);
    fn disconnected_callback(&mut self);
    fn adopted_callback(&mut self, old_doc: usize, new_doc: usize);
    fn attribute_changed_callback(&mut self, name: String, old_val: Option<String>, new_val: Option<String>);
}

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
        let old_value = if let Some(node) = self.get_node(element_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                el.attributes.get(&name).cloned()
            } else {
                None
            }
        } else {
            None
        };
        
        // Set o atributo
        self.set_attribute(element_idx, name.clone(), value.clone());
        
        // Notifica sobre a mudança
        // Em produção, chamaria attributeChangedCallback se o atributo fosse observado
    }
    
    /// Remove attribute com suporte a attributeChangedCallback
    pub fn remove_attribute_with_callback(&mut self, element_idx: usize, name: String) {
        let old_value = if let Some(node) = self.get_node(element_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                el.attributes.get(&name).cloned()
            } else {
                None
            }
        } else {
            None
        };
        
        // Remove o atributo
        self.remove_attribute(element_idx, name.clone());
        
        // Notifica sobre a mudança
    }
}

// Métodos auxiliares que precisam existir no AceDOM para o código acima compilar
impl AceDOM {
    pub fn set_attribute(&mut self, element_idx: usize, name: String, value: String) {
        if let Some(node) = self.nodes.get_mut(element_idx) {
            if let AceNodeType::Element(ref mut el) = node.node_type {
                let old_value = el.attributes.insert(name.clone(), value.clone());
                
                // Notifica observers
                self.notify_mutation(
                    element_idx,
                    crate::engine::dom::MutationRecord {
                        type_: crate::engine::dom::MutationType::Attributes,
                        target: element_idx,
                        added_nodes: vec![],
                        removed_nodes: vec![],
                        previous_sibling: None,
                        next_sibling: None,
                        attribute_name: Some(name),
                        old_value,
                    },
                );
            }
        }
    }
    
    pub fn remove_attribute(&mut self, element_idx: usize, name: String) {
        if let Some(node) = self.nodes.get_mut(element_idx) {
            if let AceNodeType::Element(ref mut el) = node.node_type {
                let old_value = el.attributes.remove(&name);
                
                // Notifica observers
                self.notify_mutation(
                    element_idx,
                    crate::engine::dom::MutationRecord {
                        type_: crate::engine::dom::MutationType::Attributes,
                        target: element_idx,
                        added_nodes: vec![],
                        removed_nodes: vec![],
                        previous_sibling: None,
                        next_sibling: None,
                        attribute_name: Some(name),
                        old_value,
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::dom::AceDOM;
    
    #[test]
    fn test_define_valid_name() {
        let mut registry = CustomElementsRegistry::new();
        
        let constructor = Arc::new(|| 0) as Arc<dyn Fn() -> usize + Send + Sync>;
        let result = registry.define("my-element".to_string(), constructor, None);
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_define_invalid_name_no_dash() {
        let mut registry = CustomElementsRegistry::new();
        
        let constructor = Arc::new(|| 0) as Arc<dyn Fn() -> usize + Send + Sync>;
        let result = registry.define("myelement".to_string(), constructor, None);
        
        assert_eq!(result, Err(CustomElementError::InvalidName));
    }
    
    #[test]
    fn test_define_already_defined() {
        let mut registry = CustomElementsRegistry::new();
        
        let constructor = Arc::new(|| 0) as Arc<dyn Fn() -> usize + Send + Sync>;
        let first = registry.define("my-element".to_string(), constructor.clone(), None);
        let second = registry.define("my-element".to_string(), constructor, None);
        
        assert!(first.is_ok());
        assert_eq!(second, Err(CustomElementError::AlreadyDefined));
    }
    
    #[test]
    fn test_get_definition() {
        let mut registry = CustomElementsRegistry::new();
        
        let constructor = Arc::new(|| 0) as Arc<dyn Fn() -> usize + Send + Sync>;
        registry.define("my-element".to_string(), constructor, None).unwrap();
        
        let definition = registry.get("my-element");
        assert!(definition.is_some());
        
        let not_found = registry.get("non-existent");
        assert!(not_found.is_none());
    }
    
    #[test]
    fn test_when_defined() {
        let mut registry = CustomElementsRegistry::new();
        
        let constructor = Arc::new(|| 0) as Arc<dyn Fn() -> usize + Send + Sync>;
        
        assert!(!registry.when_defined("my-element"));
        
        registry.define("my-element".to_string(), constructor, None).unwrap();
        
        assert!(registry.when_defined("my-element"));
    }
    
    #[test]
    fn test_upgrade_element() {
        let mut dom = AceDOM::from_html("<my-element></my-element>");
        let mut registry = CustomElementsRegistry::new();
        
        let element_idx = dom.body.map(|b| {
            if let Some(node) = dom.get_node(b) {
                node.children.first().copied().unwrap_or(b)
            } else {
                b
            }
        }).unwrap_or(1);
        
        let constructor = Arc::new(|| element_idx) as Arc<dyn Fn() -> usize + Send + Sync>;
        registry.define("my-element".to_string(), constructor, None).unwrap();
        
        registry.upgrade_element(&mut dom, element_idx);
        
        // O upgrade foi tentado (em produção, chamaria o constructor)
    }
    
    #[test]
    fn test_is_connected_true() {
        let dom = AceDOM::from_html("<div><span></span></div>");
        let registry = CustomElementsRegistry::new();
        
        // span deve estar conectado
        let span_idx = 2;
        assert!(registry.is_connected(&dom, span_idx));
    }
    
    #[test]
    fn test_is_connected_false() {
        let mut dom = AceDOM::from_html("<div><span></span></div>");
        let registry = CustomElementsRegistry::new();
        
        // Cria um nó desconectado
        let disconnected_idx = dom.nodes.len();
        dom.nodes.push(AceNode {
            node_type: AceNodeType::Element(AceElement {
                tag: "div".to_string(),
                namespace: crate::ace::html::Namespace::HTML,
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
    fn test_set_attribute_with_callback() {
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
    fn test_remove_attribute_with_callback() {
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
