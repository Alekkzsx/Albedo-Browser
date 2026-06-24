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



pub enum SwEvent {
    Execute(String),
    Dispatch(String),
    Fetch(
        RequestContext,
        tokio::sync::oneshot::Sender<InterceptResult>,
    ),
    Terminate,
}
