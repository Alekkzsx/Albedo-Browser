use super::*;
use super::mark_mutation;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use rquickjs::{prelude::Rest, Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn child_element_count(el: &Element) -> usize {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            return node
                .children
                .iter()
                .filter(|&&idx| {
                    if let Some(child) = dom.get_node(idx) {
                        matches!(child.node_type, AceNodeType::Element(_))
                    } else {
                        false
                    }
                })
                .count();
        }
    }
    0
}
