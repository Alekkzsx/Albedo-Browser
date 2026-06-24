use super::*;
use crate::ace::runtime::core::event_loop::AsyncResult;
use crate::ace::runtime::core::service_worker::{
    CacheMode, InterceptResult, RedirectMode, RequestContext,
};
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;



#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct AbortController {
    pub signal: AbortSignal,
}

#[rquickjs::methods]
impl AbortController {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            signal: AbortSignal::new(),
        }
    }
    /// TODO: add docs
    pub fn abort(&mut self) {
        self.signal.aborted = true;
    }
}
