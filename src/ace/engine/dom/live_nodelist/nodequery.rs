//! LiveNodeList - Coleções de nós que se atualizam automaticamente
//! 
//! Implementa HTMLCollection e NodeList conforme especificação WHATWG DOM.
//! Diferente de Vec<usize>, estas coleções são "live" - refletem mudanças no DOM
//! sem necessidade de re-query.

use super::*;
use std::cell::RefCell;
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};

/// Trait para tipos de queries suportados por LiveNodeList

pub trait NodeQuery {
pub(crate) fn matches(&self, node: &AceNode, dom: &AceDOM) -> bool;
}
