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
// SERVICE WORKER REGISTRATION (JS binding)
// ============================================================================

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct ServiceWorkerRegistrationJS {
    #[qjs(skip_trace)]
    pub registration: Arc<ServiceWorkerRegistration>,
}

#[rquickjs::methods]
impl ServiceWorkerRegistrationJS {
    #[qjs(get)]
    pub fn scope(&self) -> String {
        self.registration.scope.clone()
    }

    #[qjs(get)]
    pub fn script_url(&self) -> String {
        self.registration.script_url.clone()
    }

    #[qjs(get)]
    pub fn installing<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        match self.registration.get_installing() {
            Ok(Some(_sw)) => {
                // TODO: Return ServiceWorker object
                Ok(Value::new_null(ctx))
            }
            _ => Ok(Value::new_null(ctx)),
        }
    }

    #[qjs(get)]
    pub fn waiting<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        match self.registration.installed.lock() {
            Ok(installed) => {
                if installed.is_some() {
                    // TODO: Return ServiceWorker object
                    Ok(Value::new_null(ctx))
                } else {
                    Ok(Value::new_null(ctx))
                }
            }
            _ => Ok(Value::new_null(ctx)),
        }
    }

    #[qjs(get)]
    pub fn active<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        match self.registration.get_active() {
            Ok(Some(_sw)) => {
                // TODO: Return ServiceWorker object
                Ok(Value::new_null(ctx))
            }
            _ => Ok(Value::new_null(ctx)),
        }
    }

    /// registration.update(): Check for SW script updates
    pub fn update<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        // TODO: Fetch script again, compare, update if changed
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ = resolve.call::<_, ()>((self.clone(),));
        Ok(promise.into_value())
    }

    /// registration.unregister(): Uninstall this SW
    pub fn unregister<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        // TODO: Mark as redundant, remove from manager
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ = resolve.call::<_, ()>((true,));
        Ok(promise.into_value())
    }

    /// registration.showNotification(title, options)
    pub fn show_notification<'js>(
        &self,
        ctx: Ctx<'js>,
        _title: String,
        _options: rquickjs::prelude::Opt<Object<'js>>,
    ) -> JsResult<Value<'js>> {
        // TODO: Show browser notification
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ = resolve.call::<_, ()>(());
        Ok(promise.into_value())
    }

    /// registration.sync (background sync manager)
    pub fn sync<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        let sync_mgr = SyncManager {
            registration_id: self.registration.id.clone(),
            background_sync_queue: Arc::new(self.registration.background_sync_queue.clone()),
        };
        Ok(Class::instance(ctx, sync_mgr)?.into_value())
    }

    /// registration.periodicSync (periodic background sync manager)
    pub fn periodic_sync<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        let periodic_mgr = PeriodicSyncManager {
            registration_id: self.registration.id.clone(),
            periodic_sync_scheduler: Arc::new(self.registration.periodic_sync_scheduler.clone()),
        };
        Ok(Class::instance(ctx, periodic_mgr)?.into_value())
    }
}
