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
    pub fn dispatch_touch_event(&self, index: usize, type_: &str, x: f32, y: f32, identifier: i32) {
        if let Some(ref dom_arc) = *self.dom.lock().unwrap_or_else(|e| e.into_inner()) {
            let dom = dom_arc.clone();
            self.with_context(|ctx| {
                ctx.with(|ctx| {
                    use crate::ace::runtime::bindings::html::element::Element;
                    use crate::ace::runtime::bindings::html::event_subclasses::TouchEvent;

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

                        // Create TouchEvent options object
                        let opts = rquickjs::Object::new(ctx.clone()).expect("Albedo Engine: internal invariant violated");
                        let _ = opts.set("clientX", x as f64);
                        let _ = opts.set("clientY", y as f64);
                        let _ = opts.set("identifier", identifier);
                        let _ = opts.set("bubbles", true);
                        let _ = opts.set("cancelable", true);

                        if let Ok(touch_event) = rquickjs::Class::instance(
                            ctx.clone(),
                            TouchEvent::new(type_.to_string(), Some(opts.into_value())),
                        ) {
                            if let Some(obj) = instance_val.as_object() {
                                if let Ok(dispatch) =
                                    obj.get::<_, rquickjs::Function>("dispatchEvent")
                                {
                                    let _: rquickjs::Result<rquickjs::Value> =
                                        dispatch.call((touch_event,));
                                }
                            }
                        }
                    }
                })
            });
        }
    }

    /// TODO: add docs
    pub fn dispatch_message_event(
        &self,
        message_json: String,
        origin: String,
        _source_rt_id: Option<usize>,
    ) {
        tracing::debug!(runtime_id = self.id, "Dispatching message event");
        if self.dom.lock().unwrap_or_else(|e| e.into_inner()).is_some() {
            tracing::debug!("DOM exists, getting context lock");
            self.with_context(|ctx| {
                tracing::debug!("Context locked, evaluating script");
                ctx.with(|ctx| {
                    let safe_msg = message_json.replace("'", "\\'");
                    let safe_origin = origin.replace("'", "\\'");
                    let options_str =
                        format!("{{ data: '{}', origin: '{}' }}", safe_msg, safe_origin);
                    let script = format!(
                        "globalThis.dispatchEvent(new MessageEvent('message', {}))",
                        options_str
                    );

                    let _ = ctx.eval::<(), _>(script);
                })
            });
            tracing::debug!("Message event dispatch complete");
        } else {
            tracing::warn!("No DOM available for message event");
        }
    }

    /// TODO: add docs
    pub fn get_pending_navigation(&self) -> Option<String> {
        let mut pending = self.pending_navigation.lock().unwrap_or_else(|e| e.into_inner());
        pending.take()
    }

    /// TODO: add docs
    pub fn check_same_origin(&self, other: &JsRuntime) -> bool {
        let o1_lock = self.origin.lock().unwrap_or_else(|e| e.into_inner());
        let o2_lock = other.origin.lock().unwrap_or_else(|e| e.into_inner());
        match (&*o1_lock, &*o2_lock) {
            (Some(o1), Some(o2)) => o1.is_same_origin(o2),
            _ => false,
        }
    }

    /// TODO: add docs
    pub fn run_gc(&self) {
        let rt = self.runtime.lock().unwrap_or_else(|e| e.into_inner());
        rt.run_gc();
    }
}
