use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn attach_shadow<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    if let Ok(mut dom) = el.dom.lock() {
        let shadow_idx = dom.attach_shadow(el.index);

        let element = Element {
            dom: el.dom.clone(),
            index: shadow_idx,
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
    // Erro ao travar mutex
    Err(rquickjs::Error::Unknown)
}
