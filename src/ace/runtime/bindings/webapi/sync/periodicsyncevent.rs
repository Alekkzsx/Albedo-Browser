use super::*;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result as JsResult, Value};
use std::sync::{Arc, Mutex};

// ============================================================================
// SYNC EVENT (Background Sync)
// ============================================================================



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
