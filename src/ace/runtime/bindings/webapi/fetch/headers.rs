use super::*;
use crate::ace::runtime::core::event_loop::AsyncResult;
use crate::ace::runtime::core::service_worker::{
    CacheMode, InterceptResult, RedirectMode, RequestContext,
};
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;



#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Headers {
    #[qjs(skip_trace)]
    pub map: HashMap<String, String>,
}

#[rquickjs::methods]
impl Headers {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// TODO: add docs
    pub fn get(&self, name: String) -> Option<String> {
        self.map.get(&name.to_lowercase()).cloned()
    }

    /// TODO: add docs
    pub fn set(&mut self, name: String, value: String) {
        self.map.insert(name.to_lowercase(), value);
    }
}
