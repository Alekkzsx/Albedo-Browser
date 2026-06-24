use super::*;
use super::mark_mutation;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use rquickjs::{prelude::Rest, Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn next_sibling<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            if let Some(sibling_idx) = node.next_sibling {
                let element = Element {
                    dom: el.dom.clone(),
                    index: sibling_idx,
                    mutations: el.mutations.clone(),
                    stylesheet_dirty: el.stylesheet_dirty.clone(),
                    primitives: el.primitives.clone(),
                    canvas_contexts: el.canvas_contexts.clone(),
                    pending_scroll: el.pending_scroll.clone(),
                    element_geometry: el.element_geometry.clone(),
                    element_scroll: el.element_scroll.clone(),
                };
                let instance = Class::instance(ctx, element)?;
                return Ok(instance.into_value());
            }
        }
    }
    Ok(Value::new_null(ctx))
}
