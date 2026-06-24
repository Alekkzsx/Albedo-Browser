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
// PUBLIC API REGISTRATION
// ============================================================================

/// TODO: add docs
pub fn register_service_worker_container(rt: &JsRuntime) -> JsResult<()> {
    let container = ServiceWorkerContainer {
        rt: Arc::new(Mutex::new(rt.clone())),
        manager: rt.sw_manager.clone(),
        controller: Arc::new(Mutex::new(None)),
    };

    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            // Register class definitions with globals object
            let globals = ctx.globals();
            Class::<ServiceWorkerClient>::define(&globals)?;
            Class::<Clients>::define(&globals)?;
            Class::<ServiceWorkerRegistrationJS>::define(&globals)?;

            // Create navigator.serviceWorker
            let navigator = ctx.globals().get::<_, Object>("navigator")?;
            let container_obj = Class::instance(ctx, container)?;
            navigator.set("serviceWorker", container_obj)?;

            Ok(())
        })
    })
}
