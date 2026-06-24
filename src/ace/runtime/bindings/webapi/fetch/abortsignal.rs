use super::*;
use crate::ace::runtime::core::event_loop::AsyncResult;
use crate::ace::runtime::core::service_worker::{
    CacheMode, InterceptResult, RedirectMode, RequestContext,
};
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;


#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct AbortSignal {
    pub aborted: bool,
}

#[rquickjs::methods]
impl AbortSignal {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self { aborted: false }
    }
}
