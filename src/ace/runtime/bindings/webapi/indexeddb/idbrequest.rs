use super::*;
use crate::ace::runtime::bindings::webapi::idb_service::worker::IDBWorkerCommand;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};



#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IDBRequest {
    #[qjs(skip_trace)]
    pub callback_id: usize,
    #[qjs(skip_trace)]
    pub observer_registry: Arc<Mutex<HashMap<usize, Persistent<Function<'static>>>>>,
}

#[rquickjs::methods]
impl IDBRequest {
    #[qjs(get, rename = "onsuccess")]
    pub fn get_onsuccess<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onsuccess")]
    pub fn set_onsuccess<'js>(&mut self, ctx: Ctx<'js>, f: Function<'js>) {
        let p = Persistent::save(&ctx, f);
        self.observer_registry
            .lock().unwrap_or_else(|e| e.into_inner())
            .insert(self.callback_id, p);
    }

    #[qjs(get, rename = "onerror")]
    pub fn get_onerror<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onerror")]
    pub fn set_onerror<'js>(&mut self, ctx: Ctx<'js>, f: Function<'js>) {
        let p = Persistent::save(&ctx, f);
        self.observer_registry
            .lock().unwrap_or_else(|e| e.into_inner())
            .insert(self.callback_id, p);
    }
}
