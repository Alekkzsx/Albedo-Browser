use crate::ace::engine::dom::MutationObserverInit;
use crate::ace::runtime::bindings::html::element::Element;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};

#[derive(Clone)]
#[rquickjs::class]
pub struct MutationObserver {
    id: usize,
    rt: JsRuntime,
}

#[rquickjs::methods]
impl MutationObserver {
    #[qjs(constructor)]
    pub fn new<'js>(ctx: Ctx<'js>, callback: Function<'js>) -> Result<Self> {
        let rt = ctx
            .globals()
            .get::<_, JsRuntime>("__albedo_rt__")
            .expect("JsRuntime required");

        // Use a simple incrementing ID for observers
        let id = {
            let registry = rt.observer_registry.lock().unwrap();
            registry.len() + 1
        };

        // Save callback in registry (transmute to 'static Persistent for storage)
        {
            let mut registry = rt.observer_registry.lock().unwrap();
            let cb_persist = Persistent::save(&ctx, callback);
            let cb_static: Persistent<rquickjs::Function<'static>> =
                unsafe { std::mem::transmute(cb_persist) };
            registry.insert(id, cb_static);
        }

        Ok(MutationObserver { id, rt })
    }

    pub fn observe(&self, target: Value<'_>, options: Object<'_>) -> Result<()> {
        let element = Class::<Element>::from_value(&target)
            .map_err(|_| rquickjs::Error::new_from_js("Target must be an Element", "TypeError"))?;

        let node_idx = element.borrow().index;

        let init = MutationObserverInit {
            child_list: options.get("childList").unwrap_or(false),
            attributes: options.get("attributes").unwrap_or(false),
            character_data: options.get("characterData").unwrap_or(false),
            subtree: options.get("subtree").unwrap_or(false),
            attribute_old_value: options.get("attributeOldValue").unwrap_or(false),
            character_data_old_value: options.get("characterDataOldValue").unwrap_or(false),
        };

        if let Some(dom_arc) = self.rt.dom.lock().unwrap().as_ref() {
            let mut dom = dom_arc.lock().unwrap();
            dom.observe(node_idx, init, self.id);
        }

        Ok(())
    }

    pub fn disconnect(&self) {
        if let Some(dom_arc) = self.rt.dom.lock().unwrap().as_ref() {
            let mut dom = dom_arc.lock().unwrap();
            for observers in dom.observers.values_mut() {
                observers.retain(|o| o.callback_id != self.id);
            }
        }

        // Remove from registry
        let mut registry = self.rt.observer_registry.lock().unwrap();
        registry.remove(&self.id);
    }

    #[qjs(rename = "takeRecords")]
    pub fn take_records<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(dom_arc) = self.rt.dom.lock().unwrap().as_ref() {
            let dom = dom_arc.lock().unwrap();
            let records = dom
                .pending_mutations
                .borrow_mut()
                .remove(&self.id)
                .unwrap_or_default();

            let arr = rquickjs::Array::new(ctx.clone())?;
            for (i, rec) in records.into_iter().enumerate() {
                let obj = rquickjs::Object::new(ctx.clone())?;
                obj.set("type", rec.type_.as_str())?;
                obj.set("attributeName", rec.attribute_name)?;
                obj.set("oldValue", rec.old_value)?;

                let wrap_el = |idx: usize, ctx: &Ctx<'js>| -> Result<Value<'js>> {
                    let el = Element {
                        dom: dom_arc.clone(),
                        index: idx,
                        mutations: self.rt.mutations.clone(),
                        stylesheet_dirty: self.rt.stylesheet_dirty.clone(),
                        primitives: self.rt.primitives.clone(),
                        canvas_contexts: self.rt.canvas_contexts.clone(),
                        pending_scroll: self.rt.pending_scroll.clone(),
                        element_geometry: self.rt.element_geometry.clone(),
                        element_scroll: self.rt.element_scroll.clone(),
                    };
                    Ok(Class::instance(ctx.clone(), el)?.into_value())
                };

                // Wrap target
                obj.set("target", wrap_el(rec.target, &ctx)?)?;

                // addedNodes
                let added_arr = rquickjs::Array::new(ctx.clone())?;
                for (idx, &node_idx) in rec.added_nodes.iter().enumerate() {
                    added_arr.set(idx, wrap_el(node_idx, &ctx)?)?;
                }
                obj.set("addedNodes", added_arr)?;

                // removedNodes
                let removed_arr = rquickjs::Array::new(ctx.clone())?;
                for (idx, &node_idx) in rec.removed_nodes.iter().enumerate() {
                    removed_arr.set(idx, wrap_el(node_idx, &ctx)?)?;
                }
                obj.set("removedNodes", removed_arr)?;

                // Siblings
                if let Some(prev) = rec.previous_sibling {
                    obj.set("previousSibling", wrap_el(prev, &ctx)?)?;
                } else {
                    obj.set("previousSibling", rquickjs::Value::new_null(ctx.clone()))?;
                }

                if let Some(next) = rec.next_sibling {
                    obj.set("nextSibling", wrap_el(next, &ctx)?)?;
                } else {
                    obj.set("nextSibling", rquickjs::Value::new_null(ctx.clone()))?;
                }

                arr.set(i, obj)?;
            }
            return Ok(arr.into_value());
        }
        Ok(rquickjs::Array::new(ctx)?.into_value())
    }
}

impl rquickjs::class::Trace<'_> for MutationObserver {
    fn trace<'a>(&self, _tracer: rquickjs::class::Tracer<'a, '_>) {}
}
