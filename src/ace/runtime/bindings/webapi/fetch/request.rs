use super::*;
use crate::ace::runtime::core::event_loop::AsyncResult;
use crate::ace::runtime::core::service_worker::{
    CacheMode, InterceptResult, RedirectMode, RequestContext,
};
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;



#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Request {
    pub url: String,
    pub method: String,
    pub headers: Headers,
}

#[rquickjs::methods]
impl Request {
    #[qjs(constructor)]
    pub fn new(url: String, options: Option<Object<'_>>) -> Self {
        let mut method = "GET".to_string();
        if let Some(opts) = options {
            method = opts.get("method").unwrap_or("GET".to_string());
        }
        Self {
            url,
            method,
            headers: Headers::new(),
        }
    }
}
