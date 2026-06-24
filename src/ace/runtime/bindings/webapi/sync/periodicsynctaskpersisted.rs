use super::*;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result as JsResult, Value};
use std::sync::{Arc, Mutex};

// ============================================================================
// SYNC EVENT (Background Sync)
// ============================================================================



#[derive(Clone, Debug)]
pub struct PeriodicSyncTaskPersisted {
    pub id: String,
    pub tag: String,
    pub registration_id: String,
    pub min_interval_ms: u64,
    pub last_executed_ms: Option<u64>,
    pub enabled: bool,
}
