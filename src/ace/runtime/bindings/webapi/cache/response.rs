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



#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Response {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub url: String,
    pub redirected: bool,
}

#[rquickjs::methods]
impl Response {
    #[qjs(constructor)]
    pub fn new(
        body: rquickjs::prelude::Opt<String>,
        options: rquickjs::prelude::Opt<Object<'_>>,
    ) -> JsResult<Self> {
        let status = if let Some(opts) = options.0 {
            opts.get::<_, u16>("status").unwrap_or(200)
        } else {
            200
        };

        Ok(Self {
            status,
            status_text: "OK".to_string(),
            headers: HashMap::new(),
            body: body.0.unwrap_or_default().into_bytes(),
            url: String::new(),
            redirected: false,
        })
    }

    #[qjs(get)]
    pub fn status(&self) -> u16 {
        self.status
    }

    #[qjs(get)]
    pub fn status_text(&self) -> String {
        self.status_text.clone()
    }

    #[qjs(get)]
    pub fn ok(&self) -> bool {
        self.status >= 200 && self.status < 300
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
    pub async fn text<'js>(&self, _ctx: Ctx<'js>) -> JsResult<String> {
        Ok(String::from_utf8_lossy(&self.body).to_string())
    }

    /// TODO: add docs
    pub async fn json<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        // Otimização: QuickJS já tem um parser de JSON nativo rápido.
        // Passamos os bytes diretamente para o motor JS.
        ctx.json_parse(self.body.clone())
    }

    /// TODO: add docs
    pub fn clone_response(&self) -> Response {
        self.clone()
    }
}
