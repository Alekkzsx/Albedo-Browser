use super::*;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result as JsResult, Value};
use std::sync::{Arc, Mutex};

// ============================================================================
// SYNC EVENT (Background Sync)
// ============================================================================



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
