use super::*;
use crate::ace::engine::dom::AceDOM;
use crate::network::resources::ResourceManager;
use crate::shared::security::Origin;
use rquickjs::function::IntoJsFunc;
use rquickjs::{Context, Ctx, Runtime, Value};
use std::collections::HashMap;
use std::result::Result as StdResult;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};



use super::event_loop::EventLoop;

impl JsRuntime {

    /// TODO: add docs
    pub fn dispatch_keyboard_event(
        &self,
        index: usize,
        type_: &str,
        key: &str,
        code: &str,
        ctrl: bool,
        shift: bool,
        alt: bool,
        meta: bool,
    ) {
        if let Some(ref dom_arc) = *self.dom.lock().unwrap_or_else(|e| e.into_inner()) {
            let dom = dom_arc.clone();
            self.with_context(|ctx| {
                ctx.with(|ctx| {
                    use crate::ace::runtime::bindings::html::element::Element;
                    use crate::ace::runtime::bindings::html::event_subclasses::KeyboardEvent;

                    let element = Element {
                        dom,
                        index,
                        mutations: self.mutations.clone(),
                        stylesheet_dirty: self.stylesheet_dirty.clone(),
                        primitives: self.primitives.clone(),
                        canvas_contexts: self.canvas_contexts.clone(),
                        pending_scroll: self.pending_scroll.clone(),
                        element_geometry: self.element_geometry.clone(),
                        element_scroll: self.element_scroll.clone(),
                    };

                    if let Ok(instance) = rquickjs::Class::instance(ctx.clone(), element) {
                        let instance_val = instance.into_value();

                        // Create KeyboardEvent options object
                        let opts = rquickjs::Object::new(ctx.clone()).expect("Albedo Engine: internal invariant violated");
                        let _ = opts.set("key", key);
                        let _ = opts.set("code", code);
                        let _ = opts.set("ctrlKey", ctrl);
                        let _ = opts.set("shiftKey", shift);
                        let _ = opts.set("altKey", alt);
                        let _ = opts.set("metaKey", meta);
                        let _ = opts.set("bubbles", true);

                        if let Ok(kb_event) = rquickjs::Class::instance(
                            ctx.clone(),
                            KeyboardEvent::new(type_.to_string(), Some(opts.into_value())),
                        ) {
                            if let Some(obj) = instance_val.as_object() {
                                if let Ok(dispatch) =
                                    obj.get::<_, rquickjs::Function>("dispatchEvent")
                                {
                                    let _: rquickjs::Result<rquickjs::Value> =
                                        dispatch.call((kb_event,));
                                }
                            }
                        }
                    }
                })
            });
        }
    }

    /// TODO: add docs
    pub fn dispatch_pointer_event(
        &self,
        index: usize,
        type_: &str,
        x: f32,
        y: f32,
        button: i32,
        pointer_id: i32,
        pointer_type: &str,
        is_primary: bool,
    ) {
        if let Some(ref dom_arc) = *self.dom.lock().unwrap_or_else(|e| e.into_inner()) {
            let dom = dom_arc.clone();
            self.with_context(|ctx| {
                ctx.with(|ctx| {
                    use crate::ace::runtime::bindings::html::element::Element;
                    use crate::ace::runtime::bindings::html::event_subclasses::PointerEvent;

                    let element = Element {
                        dom,
                        index,
                        mutations: self.mutations.clone(),
                        stylesheet_dirty: self.stylesheet_dirty.clone(),
                        primitives: self.primitives.clone(),
                        canvas_contexts: self.canvas_contexts.clone(),
                        pending_scroll: self.pending_scroll.clone(),
                        element_geometry: self.element_geometry.clone(),
                        element_scroll: self.element_scroll.clone(),
                    };

                    if let Ok(instance) = rquickjs::Class::instance(ctx.clone(), element) {
                        let instance_val = instance.into_value();

                        // Create PointerEvent options object
                        let opts = rquickjs::Object::new(ctx.clone()).expect("Albedo Engine: internal invariant violated");
                        let _ = opts.set("clientX", x as f64);
                        let _ = opts.set("clientY", y as f64);
                        let _ = opts.set("button", button);
                        let _ = opts.set("pointerId", pointer_id);
                        let _ = opts.set("pointerType", pointer_type);
                        let _ = opts.set("isPrimary", is_primary);
                        let _ = opts.set("bubbles", true);

                        if let Ok(pointer_event) = rquickjs::Class::instance(
                            ctx.clone(),
                            PointerEvent::new(type_.to_string(), Some(opts.into_value())),
                        ) {
                            if let Some(obj) = instance_val.as_object() {
                                if let Ok(dispatch) =
                                    obj.get::<_, rquickjs::Function>("dispatchEvent")
                                {
                                    let _: rquickjs::Result<rquickjs::Value> =
                                        dispatch.call((pointer_event,));
                                }
                            }
                        }
                    }
                })
            });
        }
    }
}
