//! LiveNodeList - Coleções de nós que se atualizam automaticamente
//! 
//! Implementa HTMLCollection e NodeList conforme especificação WHATWG DOM.
//! Diferente de Vec<usize>, estas coleções são "live" - refletem mudanças no DOM
//! sem necessidade de re-query.

use super::*;
use std::cell::RefCell;
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};

/// Trait para tipos de queries suportados por LiveNodeList


/// Query por class name (suporta múltiplas classes)
#[derive(Clone, Debug)]
pub struct ClassNameQuery(pub Vec<String>);

impl NodeQuery for ClassNameQuery {
pub(crate) fn matches(&self, node: &AceNode, _dom: &AceDOM) -> bool {
        match &node.node_type {
            AceNodeType::Element(el) => {
                if let Some(class_attr) = el.attributes.get("class") {
                    let node_classes: Vec<&str> = class_attr.split_whitespace().collect();
                    self.0.iter().all(|required| {
                        node_classes.iter().any(|nc| nc == &required.as_str())
                    })
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
