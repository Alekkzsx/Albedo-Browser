use super::*;
use crate::ace::runtime::bindings::webapi::idb_service::worker::IDBWorkerCommand;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};



#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IDBOpenDBRequest {
    #[qjs(skip_trace)]
    pub request: IDBRequest,
}

#[rquickjs::methods]
impl IDBOpenDBRequest {
    #[qjs(get, rename = "onupgradeneeded")]
    pub fn get_onupgradeneeded<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onupgradeneeded")]
    pub fn set_onupgradeneeded<'js>(&mut self, ctx: Ctx<'js>, f: Function<'js>) {
        let p = Persistent::save(&ctx, f);
        self.request
            .observer_registry
            .lock().unwrap_or_else(|e| e.into_inner())
            .insert(self.request.callback_id, p);
    }

    #[qjs(get, rename = "onsuccess")]
    pub fn get_onsuccess<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onsuccess")]
    pub fn set_onsuccess<'js>(&mut self, ctx: Ctx<'js>, f: Function<'js>) {
        let p = Persistent::save(&ctx, f);
        self.request
            .observer_registry
            .lock().unwrap_or_else(|e| e.into_inner())
            .insert(self.request.callback_id, p);
    }
}
