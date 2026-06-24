use super::*;
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
