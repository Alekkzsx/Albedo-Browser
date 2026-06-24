use super::*;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result as JsResult, Value};
use std::sync::{Arc, Mutex};

// ============================================================================
// SYNC EVENT (Background Sync)
// ============================================================================



// ============================================================================
// PERIODIC SYNC MANAGER (Scheduled sync handler)
// ============================================================================

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct PeriodicSyncManager {
    #[qjs(skip_trace)]
    pub registration_id: String,
    #[qjs(skip_trace)]
    pub periodic_sync_scheduler: Arc<crate::ace::runtime::core::service_worker::PeriodicSyncScheduler>,
}

#[rquickjs::methods]
impl PeriodicSyncManager {
    /// registration.periodicSync.register(tag, options): Register periodic sync task
    pub fn register<'js>(
        &self,
        ctx: Ctx<'js>,
        tag: String,
        options: rquickjs::prelude::Opt<Object<'js>>,
    ) -> JsResult<Value<'js>> {
        // Extract minInterval from options (default 24 hours)
        let min_interval = if let Some(opts) = options.0 {
            opts.get::<_, u64>("minInterval")
                .unwrap_or(24 * 60 * 60 * 1000) // 24 hours in ms
        } else {
            24 * 60 * 60 * 1000
        };

        let reg_id = self.registration_id.clone();
        let scheduler = self.periodic_sync_scheduler.clone();

        // Register in scheduler
        match scheduler.register(tag.clone(), min_interval, reg_id) {
            Ok(_) => {
                // Return resolved Promise<void>
                let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
                let _ = resolve.call::<_, ()>(());
                Ok(promise.into_value())
            }
            Err(e) => {
                let msg =
                    Box::leak(format!("Failed to register periodic sync: {}", e).into_boxed_str());
                Err(rquickjs::Error::new_from_js("PeriodicSyncManager", msg))
            }
        }
    }

    /// registration.periodicSync.getTags(): Get all registered periodic sync tags
    pub fn get_tags<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        let tasks = self.periodic_sync_scheduler.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let tags: Vec<String> = tasks.keys().cloned().collect();

        let arr = rquickjs::Array::new(ctx.clone())?;
        for (i, tag) in tags.iter().enumerate() {
            arr.set(i, tag.clone())?;
        }

        Ok(arr.into_value())
    }

    /// registration.periodicSync.unregister(tag): Unregister periodic sync task
    pub fn unregister<'js>(&self, ctx: Ctx<'js>, tag: String) -> JsResult<Value<'js>> {
        let scheduler = self.periodic_sync_scheduler.clone();

        match scheduler.unregister(&tag) {
            Ok(_) => {
                let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
                let _ = resolve.call::<_, ()>(());
                Ok(promise.into_value())
            }
            Err(e) => {
                let msg = Box::leak(format!("Failed to unregister: {}", e).into_boxed_str());
                Err(rquickjs::Error::new_from_js("PeriodicSyncManager", msg))
            }
        }
    }
}
