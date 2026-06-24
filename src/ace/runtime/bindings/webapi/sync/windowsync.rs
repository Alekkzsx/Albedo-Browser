use super::*;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result as JsResult, Value};
use std::sync::{Arc, Mutex};

// ============================================================================
// SYNC EVENT (Background Sync)
// ============================================================================



// ============================================================================
// WINDOW SYNC INTERFACE (for standalone scripts)
// ============================================================================

pub struct WindowSync {
    pub background_sync_queue: Arc<crate::ace::runtime::core::service_worker::BackgroundSyncQueue>,
    pub periodic_sync_scheduler: Arc<crate::ace::runtime::core::service_worker::PeriodicSyncScheduler>,
}
