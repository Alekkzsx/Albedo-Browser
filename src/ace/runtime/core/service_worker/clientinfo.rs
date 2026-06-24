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



#[derive(Clone, Debug)]
pub struct ClientInfo {
    pub id: String,
    pub url: String,
    pub frame_type: String, // "top-level", "nested", "iframe", "worker"
    pub focused: bool,
}
