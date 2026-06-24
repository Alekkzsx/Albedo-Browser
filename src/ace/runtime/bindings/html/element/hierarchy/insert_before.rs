use super::*;
use super::mark_mutation;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use rquickjs::{prelude::Rest, Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn insert_before<'js>(
    el: &Element,
    _ctx: Ctx<'js>,
    child: Value<'js>,
    ref_child: Value<'js>,
) -> Result<Value<'js>> {
    let parent_idx = el.index;

    let ref_idx: Option<usize> = if ref_child.is_null() || ref_child.is_undefined() {
        None
    } else {
        let ref_el = Class::<Element>::from_value(&ref_child).map_err(|_| {
            rquickjs::Error::new_from_js("TypeError", "Argument 2 must be an Element or null")
        })?;
        let element_index = ref_el.borrow().index;
        Some(index)
    };

    if let Ok(child_el) = Class::<Element>::from_value(&child) {
        let child_idx = child_el.borrow().index;
        if let Ok(mut dom) = el.dom.lock() {
            dom.insert_before(parent_idx, child_idx, ref_idx);
        }
        mark_mutation(el);
        return Ok(child);
    } else if let Ok(fragment) = Class::<
        crate::ace::runtime::bindings::html::document::fragment::DocumentFragment,
    >::from_value(&child)
    {
        let fragment_borrow = fragment.borrow();
        let fragment_idx = fragment_borrow.index;

        if let Ok(mut dom) = el.dom.lock() {
            // Move all children from fragment to parent before ref_idx
            let children_to_move = if let Some(frag_node) = dom.get_node(fragment_idx) {
                frag_node.children.clone()
            } else {
                Vec::new()
            };

            for child_idx in children_to_move {
                dom.insert_before(parent_idx, child_idx, ref_idx);
            }
        }

        mark_mutation(el);
        return Ok(child);
    }

    Err(rquickjs::Error::new_from_js(
        "TypeError",
        "Argument 1 must be an Element or DocumentFragment",
    ))
}
