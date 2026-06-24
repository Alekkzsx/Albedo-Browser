use super::*;
//! LiveNodeList - Coleções de nós que se atualizam automaticamente
//! 
//! Implementa HTMLCollection e NodeList conforme especificação WHATWG DOM.
//! Diferente de Vec<usize>, estas coleções são "live" - refletem mudanças no DOM
//! sem necessidade de re-query.

use std::cell::RefCell;
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};

/// Trait para tipos de queries suportados por LiveNodeList


/// Query por ID (exato)
#[derive(Clone, Debug)]
pub struct IdQuery(pub String);

impl NodeQuery for IdQuery {
pub(crate) fn matches(&self, node: &AceNode, _dom: &AceDOM) -> bool {
        match &node.node_type {
            AceNodeType::Element(el) => {
                el.attributes.get("id").map_or(false, |id| id == &self.0)
            }
            _ => false,
        }
    }
}
