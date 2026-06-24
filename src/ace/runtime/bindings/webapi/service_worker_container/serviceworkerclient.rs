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


#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct ServiceWorkerClient {
    pub id: String,
    pub url: String,
    pub frame_type: String, // "top-level", "nested", "iframe", "worker"
    pub focused: bool,
}

#[rquickjs::methods]
impl ServiceWorkerClient {
    #[qjs(get)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    #[qjs(get)]
    pub fn url(&self) -> String {
        self.url.clone()
    }

    #[qjs(get)]
    pub fn frame_type(&self) -> String {
        self.frame_type.clone()
    }

    #[qjs(get)]
    pub fn focused(&self) -> bool {
        self.focused
    }

    /// TODO: add docs
    pub fn post_message<'js>(&self, _ctx: Ctx<'js>, _message: Value<'js>) -> JsResult<()> {
        // TODO: Post message to client window/tab
        Ok(())
    }
}
