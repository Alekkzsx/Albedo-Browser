use super::*;
use super::runtime::JsRuntime;
use crate::ace::runtime::bindings::webapi::indexeddb::IDBDatabase;
use rquickjs::{Class, Ctx, Value};
use std::collections::HashMap;



pub(crate) fn wrap_element<'js>(rt: &JsRuntime, node_idx: usize, ctx: &Ctx<'js>) -> rquickjs::Value<'js> {
    use crate::ace::runtime::bindings::html::element::Element;
    let el = Element {
        dom: rt.dom.clone(),
        index: node_idx,
        mutations: rt.mutations.clone(),
        stylesheet_dirty: rt.stylesheet_dirty.clone(),
        primitives: rt.primitives.clone(),
        canvas_contexts: rt.canvas_contexts.clone(),
        pending_scroll: rt.pending_scroll.clone(),
        element_geometry: rt.element_geometry.clone(),
        element_scroll: rt.element_scroll.clone(),
    };
    if let Ok(instance) = rquickjs::Class::instance(ctx.clone(), el) {
        instance.into_value()
    } else {
        rquickjs::Value::new_null(ctx.clone())
    }
}
