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
// FETCH INTERCEPTOR CHAIN (Middleware Pattern)
// ============================================================================

#[derive(Clone, Debug)]
pub enum InterceptResult {
    Handled(ResponseContext), // SW called respondWith()
    Modified(RequestContext), // SW modified request
    PassThrough,              // SW skipped
}
