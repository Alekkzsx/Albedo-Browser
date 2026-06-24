use super::*;
use super::mark_mutation;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use rquickjs::{prelude::Rest, Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn previous_element_sibling<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            let mut curr = node.prev_sibling;
            while let Some(sibling_idx) = curr {
                if let Some(sibling_node) = dom.get_node(sibling_idx) {
                    if let AceNodeType::Element(_) = sibling_node.node_type {
                        let element = Element {
                            dom: el.dom.clone(),
                            index: sibling_idx,
                            mutations: el.mutations.clone(),
                            pending_scroll: el.pending_scroll.clone(),
                            element_geometry: el.element_geometry.clone(),
                            element_scroll: el.element_scroll.clone(),
                            stylesheet_dirty: el.stylesheet_dirty.clone(),
                            primitives: el.primitives.clone(),
                            canvas_contexts: el.canvas_contexts.clone(),
                        };
                        let instance = Class::instance(ctx, element)?;
                        return Ok(instance.into_value());
                    }
                    curr = sibling_node.prev_sibling;
                } else {
                    break;
                }
            }
        }
    }
    Ok(Value::new_null(ctx))
}
