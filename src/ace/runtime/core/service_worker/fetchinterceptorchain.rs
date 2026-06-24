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



pub struct FetchInterceptorChain {
    interceptors: Vec<Box<dyn FetchInterceptor>>,
}

impl FetchInterceptorChain {
    /// TODO: add docs
    pub fn new() -> Self {
        Self {
            interceptors: Vec::new(),
        }
    }

    /// TODO: add docs
    pub fn add_interceptor(&mut self, interceptor: Box<dyn FetchInterceptor>) {
        self.interceptors.push(interceptor);
    }

    /// TODO: add docs
    pub fn clear(&mut self) {
        self.interceptors.clear();
    }

    /// TODO: add docs
    pub fn execute(&self, req: RequestContext) -> Option<ResponseContext> {
        for interceptor in &self.interceptors {
            match interceptor.before_fetch(&req) {
                InterceptResult::Handled(res) => {
                    return Some(res);
                }
                InterceptResult::Modified(_modified_req) => {
                    // TODO: handle modified request
                    continue;
                }
                InterceptResult::PassThrough => {
                    continue;
                }
            }
        }
        None
    }
}
