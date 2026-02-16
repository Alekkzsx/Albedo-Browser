use rquickjs::{Class, Ctx, Result, Value, Function, Persistent, Object};
use crate::runtime::core::runtime::JsRuntime;
use crate::runtime::bindings::html::element::Element;
use std::sync::{Arc, Mutex};

#[rquickjs::class]
#[derive(Clone, rquickjs::class::Trace)]
pub struct IntersectionObserver {
    #[qjs(skip_trace)]
    rt: JsRuntime,
    #[qjs(skip_trace)]
    callback: Persistent<Function<'static>>,
    threshold: f32,
}

#[rquickjs::methods]
impl IntersectionObserver {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>, callback: Function<'_>, options: Option<Object<'_>>) -> Result<Self> {
        let rt = ctx.userdata::<JsRuntime>().expect("JsRuntime required").clone();
        let threshold = options.and_then(|o| o.get::<_, f32>("threshold").ok()).unwrap_or(0.0);
        
        Ok(IntersectionObserver {
            rt,
            callback: Persistent::save(ctx, callback),
            threshold,
        })
    }

    pub fn observe(&self, target: Value<'_>) -> Result<()> {
        let element = Class::<Element>::from_value(&target)
            .map_err(|_| rquickjs::Error::new_from_js("Target must be an Element", "TypeError"))?;
        
        let node_idx = element.borrow().index;
        let mut registry = self.rt.intersection_registry.lock().unwrap();
        
        let observers = registry.entry(node_idx).or_insert_with(Vec::new());
        observers.push((self.callback.clone(), self.threshold));
        
        Ok(())
    }

    pub fn unobserve(&self, target: Value<'_>) -> Result<()> {
        let element = Class::<Element>::from_value(&target)
            .map_err(|_| rquickjs::Error::new_from_js("Target must be an Element", "TypeError"))?;
        
        let node_idx = element.borrow().index;
        let mut registry = self.rt.intersection_registry.lock().unwrap();
        
        if let Some(observers) = registry.get_mut(&node_idx) {
            observers.retain(|(o, _)| o.as_ptr() != self.callback.as_ptr());
        }
        
        Ok(())
    }

    pub fn disconnect(&self) {
        let mut registry = self.rt.intersection_registry.lock().unwrap();
        for observers in registry.values_mut() {
            observers.retain(|(o, _)| o.as_ptr() != self.callback.as_ptr());
        }
    }
}
