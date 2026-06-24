use super::*;
use super::mark_mutation;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use rquickjs::{prelude::Rest, Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn last_element_child<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            // Iterate reversed
            for &child_idx in node.children.iter().rev() {
                if let Some(child_node) = dom.get_node(child_idx) {
                    if let AceNodeType::Element(_) = child_node.node_type {
                        let element = Element {
                            dom: el.dom.clone(),
                            index: child_idx,
                            mutations: el.mutations.clone(),
                            stylesheet_dirty: el.stylesheet_dirty.clone(),
                            primitives: el.primitives.clone(),
                            pending_scroll: el.pending_scroll.clone(),
                            element_geometry: el.element_geometry.clone(),
                            element_scroll: el.element_scroll.clone(),
                            canvas_contexts: el.canvas_contexts.clone(),
                        };
                        let instance = Class::instance(ctx, element)?;
                        return Ok(instance.into_value());
                    }
                }
            }
        }
    }
    Ok(Value::new_null(ctx))
}
