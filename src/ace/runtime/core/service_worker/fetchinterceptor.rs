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



pub trait FetchInterceptor: Send + Sync {
pub(crate) fn before_fetch(&self, req: &RequestContext) -> InterceptResult;
pub(crate) fn after_fetch(&self, req: &RequestContext, res: &ResponseContext) -> ResponseContext;
}
