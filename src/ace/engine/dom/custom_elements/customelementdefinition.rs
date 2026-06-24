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


#[derive(Clone)]
pub struct CustomElementDefinition {
    pub name: String,
    pub constructor: Arc<dyn Fn() -> usize + Send + Sync>,
    pub local_name: String,
    pub is_value: Option<String>,
    pub observed_attributes: Vec<String>,
    pub form_associated: bool,
}
