use super::*;
use super::mark_mutation;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use rquickjs::{prelude::Rest, Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn contains(el: &Element, other: Value) -> bool {
    let other_idx = if let Ok(other_el) = Class::<Element>::from_value(&other) {
        other_el.borrow().index
    } else {
        return false;
    };

    if el.index == other_idx {
        return true;
    }

    if let Ok(dom) = el.dom.lock() {
        let mut curr = other_idx;
        while let Some(node) = dom.get_node(curr) {
            if let Some(parent) = node.parent {
                if parent == el.index {
                    return true;
                }
                curr = parent;
            } else {
                break;
            }
        }
    }
    false
}
