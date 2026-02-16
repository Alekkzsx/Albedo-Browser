use rquickjs::{Class, Ctx, Object, Result, Value, Function, Persistent};
use std::sync::{Arc, Mutex};
use crate::engine::dom::{AceDOM, MutationObserverInit};
use crate::js::JsRuntime;
use crate::js::bindings::element::Element;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct MutationObserver {
    #[qjs(skip_trace)]
    id: usize,
    #[qjs(skip_trace)]
    rt: JsRuntime,
}

#[rquickjs::methods]
impl MutationObserver {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>, callback: Function<'_>) -> Result<Self> {
        let rt = ctx.userdata::<JsRuntime>().expect("JsRuntime required").clone();
        
        // Use a simple incrementing ID for observers
        let id = {
            let registry = rt.observer_registry.lock().unwrap();
            registry.len() + 1
        };
        
        // Save callback in registry
        {
            let mut registry = rt.observer_registry.lock().unwrap();
            registry.insert(id, Persistent::save(ctx, callback));
        }

        Ok(MutationObserver {
            id,
            rt,
        })
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
            let mut dom = dom_arc.lock().unwrap();
            let records = dom.pending_mutations.remove(&self.id).unwrap_or_default();
            
            let arr = rquickjs::Array::new(ctx.clone())?;
            for (i, rec) in records.into_iter().enumerate() {
                let obj = rquickjs::Object::new(ctx.clone())?;
                obj.set("type", rec.type_)?;
                obj.set("attributeName", rec.attribute_name)?;
                obj.set("oldValue", rec.old_value)?;
                
                // Wrap target as Element
                let target_el = Element {
                    dom: dom_arc.clone(),
                    index: rec.target,
                    mutations: self.rt.mutations.clone(),
                    stylesheet_dirty: self.rt.stylesheet_dirty.clone(),
                    primitives: self.rt.primitives.clone(),
                };
                let instance = Class::instance(ctx.clone(), target_el)?;
                obj.set("target", instance)?;
                
                arr.set(i, obj)?;
            }
            return Ok(arr.into_value());
        }
        Ok(rquickjs::Array::new(ctx)?.into_value())
    }
}
