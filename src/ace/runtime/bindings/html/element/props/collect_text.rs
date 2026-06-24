use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



pub(crate) fn collect_text(dom: &AceDOM, node: &AceNode) -> String {
    let mut s = String::new();
    if let AceNodeType::Text(text) = &node.node_type {
        s.push_str(text);
    }
    for &child_idx in &node.children {
        if let Some(child) = dom.get_node(child_idx) {
            s.push_str(&collect_text(dom, child));
        }
    }
    s
}
