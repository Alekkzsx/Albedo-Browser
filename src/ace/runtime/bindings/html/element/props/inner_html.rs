use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn inner_html(el: &Element) -> String {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            let mut s = String::new();
            for &child_idx in &node.children {
                s.push_str(&dom.serialize_subtree_html(child_idx));
            }
            return s;
        }
    }
    "".to_string()
}
