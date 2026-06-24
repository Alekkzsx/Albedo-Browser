use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};


/// TODO: add docs
pub fn tag_name(el: &Element) -> String {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            if let AceNodeType::Element(element) = &node.node_type {
                return element.tag.to_uppercase();
            }
        }
    }
    "".to_string()
}
