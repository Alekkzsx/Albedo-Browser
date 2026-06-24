use super::*;
use crate::ace::runtime::core::runtime::JsRuntime;
use crate::ace::runtime::core::sw_db::{ServiceWorkerDatabase, SwCacheEntryData};
use rquickjs::{Class, Ctx, Object, Persistent, Result as JsResult, Value};
use crate::ace::json::{self, JsonValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// CACHE ENTRY STORAGE (for IndexedDB persistence)
// ============================================================================



// ============================================================================
// REQUEST/RESPONSE WRAPPERS
// ============================================================================

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Request {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub mode: String,
    pub credentials: String,
    pub cache: String,
    pub redirect: String,
}

#[rquickjs::methods]
impl Request {
    #[qjs(get)]
    pub fn url(&self) -> String {
        self.url.clone()
    }

    #[qjs(get)]
    pub fn method(&self) -> String {
        self.method.clone()
    }

    #[qjs(get)]
    pub fn headers<'js>(&self, ctx: Ctx<'js>) -> JsResult<Object<'js>> {
        let obj = rquickjs::Object::new(ctx.clone())?;
        for (k, v) in &self.headers {
            obj.set(k.clone(), v.clone())?;
        }
        Ok(obj)
    }

    /// TODO: add docs
    pub async fn body_text<'js>(&self, _ctx: Ctx<'js>) -> JsResult<String> {
        match &self.body {
            Some(b) => Ok(String::from_utf8_lossy(b).to_string()),
            None => Ok(String::new()),
        }
    }
}
