use super::*;
use super::mark_mutation;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use rquickjs::{prelude::Rest, Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn insert_adjacent_element<'js>(
    el: &Element,
    _ctx: Ctx<'js>,
    position: String,
    element: Class<'js, Element>,
) -> Result<Class<'js, Element>> {
    let child_idx = element.borrow().index;

    if let Ok(mut dom) = el.dom.lock() {
        // MDN: insertAdjacentElement(position, element)
        let pos = position.to_lowercase();

        // Ensure element is removed from its current parent first
        dom.remove_node_from_parent(child_idx);

        match pos.as_str() {
            "beforebegin" => {
                if let Some(parent) = dom.get_node(el.index).and_then(|n| n.parent) {
                    dom.insert_before(parent, child_idx, Some(el.index));
                } else {
                    return Err(rquickjs::Error::new_from_js("DOMException", "Cannot insert before node with no parent"));
                }
            }
            "afterbegin" => {
                let first_child = dom.get_node(el.index).and_then(|n| n.children.first().cloned());
                dom.insert_before(el.index, child_idx, first_child);
            }
            "beforeend" => {
                dom.append_child(el.index, child_idx);
            }
            "afterend" => {
                if let Some(parent) = dom.get_node(el.index).and_then(|n| n.parent) {
                    let next_sibling = dom.get_node(el.index).and_then(|n| n.next_sibling);
                    dom.insert_before(parent, child_idx, next_sibling);
                } else {
                    return Err(rquickjs::Error::new_from_js("DOMException", "Cannot insert after node with no parent"));
                }
            }
            _ => {
                return Err(rquickjs::Error::new_from_js("DOMException", "Invalid position"));
            }
        }
    }

    mark_mutation(el);
    Ok(element)
}
