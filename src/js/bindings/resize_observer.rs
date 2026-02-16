use rquickjs::{Class, Ctx, Result, Value, Function, Persistent};
use crate::js::JsRuntime;
use crate::js::bindings::element::Element;
use std::sync::{Arc, Mutex};

#[rquickjs::class]
#[derive(Clone, rquickjs::class::Trace)]
pub struct ResizeObserver {
    #[qjs(skip_trace)]
    rt: JsRuntime,
    #[qjs(skip_trace)]
    callback: Persistent<Function<'static>>,
}

#[rquickjs::methods]
impl ResizeObserver {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>, callback: Function<'_>) -> Result<Self> {
        let rt = ctx.userdata::<JsRuntime>().expect("JsRuntime required").clone();
        Ok(ResizeObserver {
            rt,
            callback: Persistent::save(ctx, callback),
        })
    }

    pub fn observe(&self, target: Value<'_>) -> Result<()> {
        let element = Class::<Element>::from_value(&target)
            .map_err(|_| rquickjs::Error::new_from_js("Target must be an Element", "TypeError"))?;
        
        let node_idx = element.borrow().index;
        let mut registry = self.rt.resize_registry.lock().unwrap();
        
        let observers = registry.entry(node_idx).or_insert_with(Vec::new());
        observers.push(self.callback.clone());
        
        Ok(())
    }

    pub fn unobserve(&self, target: Value<'_>) -> Result<()> {
        let element = Class::<Element>::from_value(&target)
            .map_err(|_| rquickjs::Error::new_from_js("Target must be an Element", "TypeError"))?;
        
        let node_idx = element.borrow().index;
        let mut registry = self.rt.resize_registry.lock().unwrap();
        
        if let Some(observers) = registry.get_mut(&node_idx) {
            observers.retain(|o| o.as_ptr() != self.callback.as_ptr());
        }
        
        Ok(())
    }

    pub fn disconnect(&self) {
        let mut registry = self.rt.resize_registry.lock().unwrap();
        for observers in registry.values_mut() {
            observers.retain(|o| o.as_ptr() != self.callback.as_ptr());
        }
    }
}
