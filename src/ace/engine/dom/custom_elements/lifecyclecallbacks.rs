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


/// Lifecycle Callbacks Traits
pub trait LifecycleCallbacks {
pub(crate) fn connected_callback(&mut self);
pub(crate) fn disconnected_callback(&mut self);
pub(crate) fn adopted_callback(&mut self, old_doc: usize, new_doc: usize);
pub(crate) fn attribute_changed_callback(&mut self, name: String, old_val: Option<String>, new_val: Option<String>);
}
