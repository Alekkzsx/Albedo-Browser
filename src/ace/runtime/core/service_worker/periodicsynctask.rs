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
// PERIODIC SYNC SCHEDULER
// ============================================================================

#[derive(Clone, Debug)]
pub struct PeriodicSyncTask {
    pub id: String,
    pub tag: String,
    pub registration_id: String,
    pub min_interval_ms: u64,
    pub last_executed: Option<Instant>,
    pub enabled: bool,
}
