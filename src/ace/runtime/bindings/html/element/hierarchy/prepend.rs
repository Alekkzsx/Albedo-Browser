use super::*;
use super::mark_mutation;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use rquickjs::{prelude::Rest, Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn prepend<'js>(el: &Element, ctx: Ctx<'js>, nodes: Rest<Value<'js>>) -> Result<()> {
    let mut ref_idx = None;
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            ref_idx = node.children.first().cloned();
        }
    }

    let ref_val = if let Some(idx) = ref_idx {
        let element = Element {
            dom: el.dom.clone(),
            index: idx,
            mutations: el.mutations.clone(),
            stylesheet_dirty: el.stylesheet_dirty.clone(),
            primitives: el.primitives.clone(),
            canvas_contexts: el.canvas_contexts.clone(),
            pending_scroll: el.pending_scroll.clone(),
            element_geometry: el.element_geometry.clone(),
            element_scroll: el.element_scroll.clone(),
        };
        Class::instance(ctx.clone(), element)?.into_value()
    } else {
        Value::new_null(ctx.clone())
    };

    for val in nodes.0 {
        insert_before(el, ctx.clone(), val, ref_val.clone())?;
    }
    Ok(())
}
