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
// REQUEST/RESPONSE CONTEXT
// ============================================================================

#[derive(Clone, Debug)]
pub struct RequestContext {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub mode: String,        // cors, no-cors, same-origin, navigate
    pub credentials: String, // omit, same-origin, include
    pub cache_mode: CacheMode,
    pub redirect: RedirectMode,
}
