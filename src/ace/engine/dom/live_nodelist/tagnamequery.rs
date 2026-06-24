use super::*;
//! LiveNodeList - Coleções de nós que se atualizam automaticamente
//! 
//! Implementa HTMLCollection e NodeList conforme especificação WHATWG DOM.
//! Diferente de Vec<usize>, estas coleções são "live" - refletem mudanças no DOM
//! sem necessidade de re-query.

use std::cell::RefCell;
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};

/// Trait para tipos de queries suportados por LiveNodeList


/// Query por tag name (case-insensitive para HTML)
#[derive(Clone, Debug)]
pub struct TagNameQuery(pub String);

impl NodeQuery for TagNameQuery {
pub(crate) fn matches(&self, node: &AceNode, _dom: &AceDOM) -> bool {
        match &node.node_type {
            AceNodeType::Element(el) => {
                el.tag.eq_ignore_ascii_case(&self.0) || self.0 == "*"
            }
            _ => false,
        }
    }
}
