use super::*;
use super::mark_mutation;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use rquickjs::{prelude::Rest, Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn remove_child<'js>(
    el: &Element,
    _ctx: Ctx<'js>,
    child: Class<'js, Element>,
) -> Result<Class<'js, Element>> {
    let child_borrow = child.borrow();
    let child_idx = child_borrow.index;

    if let Ok(mut dom) = el.dom.lock() {
        let is_child = dom
            .nodes
            .get(child_idx)
            .map(|n| n.parent == Some(el.index))
            .unwrap_or(false);

        if is_child {
            dom.remove_node_from_parent(child_idx);
            mark_mutation(el);
            return Ok(child.clone());
        }
    }

    Err(rquickjs::Error::new_from_js(
        "NotFoundError",
        "The node to be removed is not a child of this node",
    ))
}
