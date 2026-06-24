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
// SERVICE WORKER CONTAINER (navigator.serviceWorker)
// ============================================================================

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct ServiceWorkerContainer {
    #[qjs(skip_trace)]
    pub rt: Arc<Mutex<JsRuntime>>,
    #[qjs(skip_trace)]
    pub manager: Arc<ServiceWorkerManager>,
    #[qjs(skip_trace)]
    pub controller: Arc<Mutex<Option<Arc<ServiceWorkerInstance>>>>,
}

#[rquickjs::methods]
impl ServiceWorkerContainer {
    /// navigator.serviceWorker.register(scriptURL, options)
    pub fn register<'js>(
        &self,
        ctx: Ctx<'js>,
        script_url: String,
        options: rquickjs::prelude::Opt<Object<'js>>,
    ) -> JsResult<Value<'js>> {
        // Extract scope from options
        let scope = if let Some(opts) = options.0 {
            opts.get::<_, String>("scope")
                .unwrap_or_else(|_| "/".to_string())
        } else {
            "/".to_string()
        };

        // Validate same-origin
        let rt_lock = self.rt.lock().unwrap_or_else(|e| e.into_inner());
        let origin = rt_lock
            .origin
            .lock().unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|o| o.to_string())
            .unwrap_or_else(|| "null".to_string());

        // Create registration
        let sw_db = self.manager.db.clone();
        drop(rt_lock);

        let reg = Arc::new(ServiceWorkerRegistration::new(
            scope,
            script_url.clone(),
            origin.clone(),
            sw_db,
        ));

        // Register in manager
        let _ = self.manager.register(reg.clone());

        // Trigger installation (Asynchronous)
        let mgr_clone = self.manager.clone();
        let reg_clone = reg.clone();
        std::thread::spawn(move || {
            // This would normally be handled by a more robust scheduler
            // For now, let's just trigger it.
            let _ = mgr_clone.install_worker(reg_clone);
        });

        // TODO: Fetch script, parse, create SW instance
        // For now, return promise that resolves to registration

        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;

        let reg_js = ServiceWorkerRegistrationJS { registration: reg };

        let _ = resolve.call::<_, ()>((reg_js,));

        Ok(promise.into_value())
    }

    /// navigator.serviceWorker.getRegistrations()
    pub fn get_registrations<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        let rt_lock = self.rt.lock().unwrap_or_else(|e| e.into_inner());
        let origin = rt_lock
            .origin
            .lock().unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|o| o.to_string())
            .unwrap_or_else(|| "null".to_string());
        drop(rt_lock);

        match self.manager.get_all_for_origin(&origin) {
            Ok(registrations) => {
                let arr = rquickjs::Array::new(ctx.clone())?;
                for (i, reg) in registrations.iter().enumerate() {
                    let reg_js = ServiceWorkerRegistrationJS {
                        registration: reg.clone(),
                    };
                    arr.set(i, reg_js)?;
                }

                let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
                let reg_js_vec: Vec<ServiceWorkerRegistrationJS> = registrations
                    .iter()
                    .map(|r| ServiceWorkerRegistrationJS {
                        registration: r.clone(),
                    })
                    .collect();
                let _ = resolve.call::<(Vec<ServiceWorkerRegistrationJS>,), ()>((reg_js_vec,));

                Ok(promise.into_value())
            }
            Err(_) => {
                let arr = rquickjs::Array::new(ctx.clone())?;
                Ok(arr.into_value())
            }
        }
    }

    /// navigator.serviceWorker.getRegistration(clientURL)
    pub fn get_registration<'js>(&self, ctx: Ctx<'js>, client_url: String) -> JsResult<Value<'js>> {
        let rt_lock = self.rt.lock().unwrap_or_else(|e| e.into_inner());
        let origin = rt_lock
            .origin
            .lock().unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|o| o.to_string())
            .unwrap_or_else(|| "null".to_string());
        drop(rt_lock);

        match self.manager.find_for_url(&origin, &client_url) {
            Ok(Some(reg)) => {
                let reg_js = ServiceWorkerRegistrationJS { registration: reg };

                let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
                let _ = resolve.call::<_, ()>((reg_js,));

                Ok(promise.into_value())
            }
            _ => {
                let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
                let _ = resolve.call::<_, ()>((None::<ServiceWorkerRegistrationJS>,));

                Ok(promise.into_value())
            }
        }
    }

    /// navigator.serviceWorker.controller (read-only)
    #[qjs(get)]
    pub fn controller<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        match self.controller.lock() {
            Ok(controller) => {
                if controller.is_some() {
                    // TODO: Return ServiceWorker object
                    Ok(Value::new_null(ctx))
                } else {
                    Ok(Value::new_null(ctx))
                }
            }
            _ => Ok(Value::new_null(ctx)),
        }
    }

    /// navigator.serviceWorker.ready (read-only Promise)
    #[qjs(get)]
    pub fn ready<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        // Return promise that resolves when a SW is activated
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;

        // TODO: Resolve when controller is set to activated state
        // For now, resolve immediately

        let rt_lock = self.rt.lock().unwrap_or_else(|e| e.into_inner());
        let origin = rt_lock
            .origin
            .lock().unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|o| o.to_string())
            .unwrap_or_else(|| "null".to_string());
        drop(rt_lock);

        if let Ok(Some(reg)) = self.manager.find_for_url(&origin, "/") {
            let reg_js = ServiceWorkerRegistrationJS { registration: reg };
            let _ = resolve.call::<_, ()>((reg_js,));
        }

        Ok(promise.into_value())
    }

    /// navigator.serviceWorker.oncontrollerchange (event)
    pub fn oncontrollerchange<'js>(&self, _ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        // Return null for now - event handlers would be set via property assignment
        Ok(Value::new_null(_ctx))
    }
}
