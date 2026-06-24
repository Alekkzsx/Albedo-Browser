use super::*;
use super::mark_mutation;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use rquickjs::{prelude::Rest, Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn append_child_generic<'js>(
    dom_mutex: &Arc<Mutex<crate::ace::engine::dom::AceDOM>>,
    parent_idx: usize,
    mutations: &Arc<Mutex<bool>>,
    _ctx: Ctx<'js>,
    child: Value<'js>,
) -> Result<Value<'js>> {
    if let Ok(child_el) = Class::<Element>::from_value(&child) {
        let child_borrow = child_el.borrow();
        let child_idx = child_borrow.index;

        if let Ok(mut dom) = dom_mutex.lock() {
            dom.append_child(parent_idx, child_idx);
        }

        if let Ok(mut m) = mutations.lock() {
            *m = true;
        }
        return Ok(child);
    } else if let Ok(fragment) = Class::<
        crate::ace::runtime::bindings::html::document::fragment::DocumentFragment,
    >::from_value(&child)
    {
        let fragment_borrow = fragment.borrow();
        let fragment_idx = fragment_borrow.index;

        if let Ok(mut dom) = dom_mutex.lock() {
            let children_to_move = if let Some(frag_node) = dom.get_node(fragment_idx) {
                frag_node.children.clone()
            } else {
                Vec::new()
            };

            for child_idx in children_to_move {
                dom.append_child(parent_idx, child_idx);
            }
        }

        if let Ok(mut m) = mutations.lock() {
            *m = true;
        }
        return Ok(child);
    }

    Err(rquickjs::Error::new_from_js(
        "TypeError",
        "Argument 1 must be an Element or DocumentFragment",
    ))
}
