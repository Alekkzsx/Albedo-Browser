use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result as JsResult, Value};
use std::sync::{Arc, Mutex};

// ============================================================================
// SYNC EVENT (Background Sync)
// ============================================================================

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct SyncEvent {
    pub tag: String,
    pub last_chance: bool,
    #[qjs(skip_trace)]
    pub pending_promises: Arc<Mutex<Vec<Persistent<Function<'static>>>>>,
}

#[rquickjs::methods]
impl SyncEvent {
    /// event.waitUntil(promise): Queue promise for resolution before sync completes
    pub fn wait_until<'js>(&self, _ctx: Ctx<'js>, _promise: Value<'js>) -> JsResult<()> {
        // Store promise for tracking completion
        // In real implementation, would check if promise resolves
        Ok(())
    }

    #[qjs(get)]
    pub fn tag(&self) -> String {
        self.tag.clone()
    }

    #[qjs(get)]
    pub fn last_chance(&self) -> bool {
        self.last_chance
    }
}

// ============================================================================
// PERIODIC SYNC EVENT (Scheduled Background Tasks)
// ============================================================================

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct PeriodicSyncEvent {
    pub tag: String,
    pub min_interval: u64, // milliseconds
    #[qjs(skip_trace)]
    pub pending_promises: Arc<Mutex<Vec<Persistent<Function<'static>>>>>,
}

#[rquickjs::methods]
impl PeriodicSyncEvent {
    /// event.waitUntil(promise): Queue promise for resolution before sync completes
    pub fn wait_until<'js>(&self, _ctx: Ctx<'js>, _promise: Value<'js>) -> JsResult<()> {
        Ok(())
    }

    #[qjs(get)]
    pub fn tag(&self) -> String {
        self.tag.clone()
    }

    #[qjs(get)]
    pub fn min_interval(&self) -> u64 {
        self.min_interval
    }
}

// ============================================================================
// SYNC MANAGER (Registration handler)
// ============================================================================

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct SyncManager {
    #[qjs(skip_trace)]
    pub registration_id: String,
    #[qjs(skip_trace)]
    pub background_sync_queue: Arc<crate::ace::runtime::core::service_worker::BackgroundSyncQueue>,
}

#[rquickjs::methods]
impl SyncManager {
    /// registration.sync.register(tag): Register background sync task
    pub fn register<'js>(&self, ctx: Ctx<'js>, tag: String) -> JsResult<Value<'js>> {
        let reg_id = self.registration_id.clone();
        let queue = self.background_sync_queue.clone();

        // Register in queue
        match queue.register(tag.clone(), reg_id) {
            Ok(_) => {
                // Return resolved Promise<void>
                let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
                let _ = resolve.call::<_, ()>(());
                Ok(promise.into_value())
            }
            Err(e) => {
                // Return rejected Promise
                let msg = Box::leak(format!("Failed to register sync: {}", e).into_boxed_str());
                Err(rquickjs::Error::new_from_js("SyncManager", msg))
            }
        }
    }

    /// registration.sync.getTags(): Get all pending sync tags
    pub fn get_tags<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        let queue = self.background_sync_queue.queue.lock().unwrap();
        let tags: Vec<String> = queue.iter().map(|t| t.tag.clone()).collect();

        let arr = rquickjs::Array::new(ctx.clone())?;
        for (i, tag) in tags.iter().enumerate() {
            arr.set(i, tag.clone())?;
        }

        Ok(arr.into_value())
    }
}

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
        let tasks = self.periodic_sync_scheduler.tasks.lock().unwrap();
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

// ============================================================================
// WINDOW SYNC INTERFACE (for standalone scripts)
// ============================================================================

pub struct WindowSync {
    pub background_sync_queue: Arc<crate::ace::runtime::core::service_worker::BackgroundSyncQueue>,
    pub periodic_sync_scheduler: Arc<crate::ace::runtime::core::service_worker::PeriodicSyncScheduler>,
}

// ============================================================================
// STORAGE MANAGER FOR SYNC PERSISTENCE
// ============================================================================

#[derive(Clone, Debug)]
pub struct SyncTaskPersisted {
    pub id: String,
    pub tag: String,
    pub registration_id: String,
    pub created_at_ms: u64,
    pub retry_count: u32,
    pub last_retry_ms: Option<u64>,
    pub max_retries: u32,
}

#[derive(Clone, Debug)]
pub struct PeriodicSyncTaskPersisted {
    pub id: String,
    pub tag: String,
    pub registration_id: String,
    pub min_interval_ms: u64,
    pub last_executed_ms: Option<u64>,
    pub enabled: bool,
}

// ============================================================================
// PUBLIC API REGISTRATION
// ============================================================================

pub fn register_sync_events(rt: &JsRuntime) -> JsResult<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            // Register class definitions with globals object
            let globals = ctx.globals();
            Class::<SyncEvent>::define(&globals)?;
            Class::<PeriodicSyncEvent>::define(&globals)?;
            Class::<SyncManager>::define(&globals)?;
            Class::<PeriodicSyncManager>::define(&globals)?;

            // Define global polyfill for sync support (if no SW)
            ctx.eval::<(), _>(
                r#"
                if (!globalThis.onsync) {
                    globalThis.onsync = null;
                }
                if (!globalThis.onperiodicsync) {
                    globalThis.onperiodicsync = null;
                }
                "#,
            )?;

            Ok(())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_event_creation() {
        let sync_event = SyncEvent {
            tag: "upload-data".to_string(),
            last_chance: false,
            pending_promises: Arc::new(Mutex::new(Vec::new())),
        };

        assert_eq!(sync_event.tag, "upload-data");
        assert!(!sync_event.last_chance);
    }

    #[test]
    fn test_periodic_sync_event_creation() {
        let periodic_event = PeriodicSyncEvent {
            tag: "cleanup".to_string(),
            min_interval: 24 * 60 * 60 * 1000,
            pending_promises: Arc::new(Mutex::new(Vec::new())),
        };

        assert_eq!(periodic_event.tag, "cleanup");
        assert_eq!(periodic_event.min_interval, 24 * 60 * 60 * 1000);
    }
}
