use super::*;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result as JsResult, Value};
use std::sync::{Arc, Mutex};

// ============================================================================
// SYNC EVENT (Background Sync)
// ============================================================================



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
        let queue = self.background_sync_queue.queue.lock().unwrap_or_else(|e| e.into_inner());
        let tags: Vec<String> = queue.iter().map(|t| t.tag.clone()).collect();

        let arr = rquickjs::Array::new(ctx.clone())?;
        for (i, tag) in tags.iter().enumerate() {
            arr.set(i, tag.clone())?;
        }

        Ok(arr.into_value())
    }
}
