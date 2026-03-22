use crate::runtime::bindings::webapi::sync::{PeriodicSyncManager, SyncManager};
use crate::runtime::core::runtime::JsRuntime;
use crate::runtime::core::service_worker::{
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

    pub fn post_message<'js>(&self, _ctx: Ctx<'js>, _message: Value<'js>) -> JsResult<()> {
        // TODO: Post message to client window/tab
        Ok(())
    }
}

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
        let rt_lock = self.rt.lock().unwrap();
        let origin = rt_lock
            .origin
            .lock()
            .unwrap()
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
        let rt_lock = self.rt.lock().unwrap();
        let origin = rt_lock
            .origin
            .lock()
            .unwrap()
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
        let rt_lock = self.rt.lock().unwrap();
        let origin = rt_lock
            .origin
            .lock()
            .unwrap()
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

        let rt_lock = self.rt.lock().unwrap();
        let origin = rt_lock
            .origin
            .lock()
            .unwrap()
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

// ============================================================================
// PUBLIC API REGISTRATION
// ============================================================================

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw_client_creation() {
        let client = ServiceWorkerClient {
            id: "client-1".to_string(),
            url: "http://example.com".to_string(),
            frame_type: "top-level".to_string(),
            focused: true,
        };

        assert_eq!(client.id, "client-1");
        assert_eq!(client.frame_type, "top-level");
    }
}
