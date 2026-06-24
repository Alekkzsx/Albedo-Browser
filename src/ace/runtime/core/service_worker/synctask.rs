use super::*;
use rquickjs::Result as JsResult;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crate::utils::uuid::Uuid;

use crate::ace::runtime::core::runtime::JsRuntime;
use crate::ace::runtime::core::sw_db::{ServiceWorkerDatabase, SwRegistrationData, SwSyncTaskData};

// ============================================================================
// ENUMS & BASIC TYPES
// ============================================================================



// ============================================================================
// BACKGROUND SYNC QUEUE
// ============================================================================

#[derive(Clone, Debug)]
pub struct SyncTask {
    pub id: String,
    pub tag: String,
    pub registration_id: String,
    pub created_at: Instant,
    pub retry_count: u32,
    pub last_retry: Option<Instant>,
    pub max_retries: u32,
}
