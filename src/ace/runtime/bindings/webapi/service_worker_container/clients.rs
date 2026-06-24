use super::*;
use crate::ace::runtime::bindings::webapi::sync::{PeriodicSyncManager, SyncManager};
use crate::ace::runtime::core::runtime::JsRuntime;
use crate::ace::runtime::core::service_worker::{
    ServiceWorkerInstance, ServiceWorkerManager, ServiceWorkerRegistration,
};
use rquickjs::{Class, Ctx, Object, Result as JsResult, Value};
use std::sync::{Arc, Mutex};

// ============================================================================
// SERVICE WORKER CLIENT INFO (for clients.matchAll, etc)
// ============================================================================



// ============================================================================
// CLIENTS INTERFACE (sw.clients.matchAll, etc)
// ============================================================================

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Clients {
    #[qjs(skip_trace)]
    pub sw_instance: Arc<ServiceWorkerInstance>,
}

#[rquickjs::methods]
impl Clients {
    /// clients.matchAll(options): Get all client windows/tabs
    pub fn match_all<'js>(
        &self,
        ctx: Ctx<'js>,
        _options: rquickjs::prelude::Opt<Object<'js>>,
    ) -> JsResult<Value<'js>> {
        // TODO: Return actual clients
        let _arr = rquickjs::Array::new(ctx.clone())?;
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ = resolve.call::<(Vec<ServiceWorkerClient>,), ()>((Vec::new(),));
        Ok(promise.into_value())
    }

    /// clients.get(id): Get specific client by ID
    pub fn get<'js>(&self, ctx: Ctx<'js>, _id: String) -> JsResult<Value<'js>> {
        // TODO: Return specific client or null
        Ok(Value::new_null(ctx))
    }

    /// clients.openWindow(url): Open new window
    pub fn open_window<'js>(&self, ctx: Ctx<'js>, _url: String) -> JsResult<Value<'js>> {
        // TODO: Request tab manager to open new window
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ = resolve.call::<_, ()>((ServiceWorkerClient {
            id: "client-1".to_string(),
            url: "http://example.com".to_string(),
            frame_type: "top-level".to_string(),
            focused: true,
        },));
        Ok(promise.into_value())
    }

    /// clients.claim(): Take control of all matched clients
    pub fn claim<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ = resolve.call::<_, ()>(());
        Ok(promise.into_value())
    }
}
