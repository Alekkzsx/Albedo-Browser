use rquickjs::{Class, Ctx, Function, Persistent, Result as JsResult, Value, Object, prelude::*};
use crate::runtime::core::runtime::JsRuntime;
use crate::runtime::bindings::webapi::idb_service::worker::IDBWorkerCommand;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

// ============================================================================
// CACHE ENTRY STORAGE (for IndexedDB persistence)
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub url: String,
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub timestamp: u64,  // unix timestamp
    pub expires_at: Option<u64>,  // unix timestamp or None = no expiry
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub name: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub origin: String,
}

// ============================================================================
// CACHE CLASS (JavaScript binding)
// ============================================================================

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Cache {
    pub name: String,
    pub origin: String,
    #[qjs(skip_trace)]
    pub idb_worker: Arc<tokio::sync::mpsc::UnboundedSender<IDBWorkerCommand>>,
    #[qjs(skip_trace)]
    pub pending_requests: Arc<Mutex<HashMap<u32, CacheOperation>>>,
}

#[derive(Clone, Debug)]
enum CacheOperation {
    Add(String),
    Put(String, Vec<u8>),
    Match(String),
    Delete(String),
    Keys,
}

#[rquickjs::methods]
impl Cache {
    /// cache.add(url): Fetch from network and store in cache
    pub fn add<'js>(&self, ctx: Ctx<'js>, url: String) -> JsResult<Value<'js>> {
        // TODO: Implement actual fetch + cache storage
        // For now, return resolved promise
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;

        // Placeholder: immediately resolve
        let _ = resolve.call::<(), ()>(());

        Ok(promise.into_value())
    }

    /// cache.put(request, response): Explicitly cache request/response pair
    pub fn put<'js>(
        &self,
        ctx: Ctx<'js>,
        _request: Object<'js>,
        _response: Object<'js>,
    ) -> JsResult<Value<'js>> {
        // TODO: Parse request and response objects
        // Store in IndexedDB with proper serialization
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ = resolve.call::<(), ()>(());
        Ok(promise.into_value())
    }

    /// cache.match(request, options): Look up request in cache
    pub fn match_url<'js>(
        &self,
        ctx: Ctx<'js>,
        url: String,
        _options: rquickjs::prelude::Opt<Object<'js>>,
    ) -> JsResult<Value<'js>> {
        // TODO: Query IndexedDB for cached entry
        // Return Response object or null
        Ok(Value::new_null(ctx))
    }

    /// cache.delete(request): Delete entry from cache
    pub fn delete<'js>(&self, ctx: Ctx<'js>, url: String) -> JsResult<Value<'js>> {
        // TODO: Remove from IndexedDB
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ = resolve.call::<(bool,), ()>((true,));
        Ok(promise.into_value())
    }

    /// cache.keys(request, options): List all URLs in cache
    pub fn keys<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        // TODO: Query IndexedDB for all URLs in this cache
        let arr = rquickjs::Array::new(ctx.clone())?;
        Ok(arr.into_value())
    }
}

// ============================================================================
// CACHE STORAGE CLASS (JavaScript binding)
// ============================================================================

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct CacheStorage {
    pub origin: String,
    #[qjs(skip_trace)]
    pub idb_worker: Arc<tokio::sync::mpsc::UnboundedSender<IDBWorkerCommand>>,
    #[qjs(skip_trace)]
    pub cache_list: Arc<Mutex<HashMap<String, Cache>>>,
}

#[rquickjs::methods]
impl CacheStorage {
    /// caches.open(name): Open/create cache with given name
    pub fn open<'js>(&self, ctx: Ctx<'js>, name: String) -> JsResult<Value<'js>> {
        // Check if already in memory
        let mut cache_list = self
            .cache_list
            .lock()
            .map_err(|_| rquickjs::Error::new_from_js("CacheStorage", "Failed to lock cache list"))?;

        if let Some(cache) = cache_list.get(&name) {
            let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
            let _ = resolve.call::<(Cache,), ()>((cache.clone(),));
            return Ok(promise.into_value());
        }

        // Create new cache
        let cache = Cache {
            name: name.clone(),
            origin: self.origin.clone(),
            idb_worker: self.idb_worker.clone(),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        };

        cache_list.insert(name, cache.clone());

        // Return resolved Promise<Cache>
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ = resolve.call::<(Cache,), ()>((cache,));

        Ok(promise.into_value())
    }

    /// caches.keys(): List all cache names
    pub fn keys<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        let cache_list = self
            .cache_list
            .lock()
            .map_err(|_| rquickjs::Error::new_from_js("CacheStorage", "Failed to lock cache list"))?;

        let arr = rquickjs::Array::new(ctx.clone())?;
        for (i, name) in cache_list.keys().enumerate() {
            arr.set(i, name.clone())?;
        }

        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ = resolve.call::<(Vec<String>,), ()>((cache_list.keys().cloned().collect::<Vec<_>>(),));

        Ok(promise.into_value())
    }

    /// caches.delete(name): Delete entire cache
    pub fn delete<'js>(&self, ctx: Ctx<'js>, name: String) -> JsResult<Value<'js>> {
        let mut cache_list = self
            .cache_list
            .lock()
            .map_err(|_| rquickjs::Error::new_from_js("CacheStorage", "Failed to lock cache list"))?;

        let existed = cache_list.remove(&name).is_some();

        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ = resolve.call::<(bool,), ()>((existed,));

        Ok(promise.into_value())
    }

    /// caches.has(name): Check if cache exists
    pub fn has(&self, name: String) -> bool {
        match self.cache_list.lock() {
            Ok(cache_list) => cache_list.contains_key(&name),
            Err(_) => false,
        }
    }
}

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

    pub async fn body_text<'js>(&self, _ctx: Ctx<'js>) -> JsResult<String> {
        match &self.body {
            Some(b) => Ok(String::from_utf8_lossy(b).to_string()),
            None => Ok(String::new()),
        }
    }
}

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

    pub async fn text<'js>(&self, _ctx: Ctx<'js>) -> JsResult<String> {
        Ok(String::from_utf8_lossy(&self.body).to_string())
    }

    pub async fn json<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        let json_str = String::from_utf8_lossy(&self.body);
        let json_val: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|_| rquickjs::Error::new_from_js("Response", "JSON parse error"))?;

        // Convert serde_json::Value to rquickjs Value
        let json_bytes = serde_json::to_string(&json_val).unwrap().into_bytes();
        ctx.json_parse(json_bytes)
    }

    pub fn clone_response(&self) -> Response {
        self.clone()
    }
}

// ============================================================================
// PUBLIC API REGISTRATION
// ============================================================================

pub fn register_cache_storage(rt: &JsRuntime) -> JsResult<()> {
    let origin = rt
        .origin
        .lock()
        .unwrap()
        .as_ref()
        .map(|o| o.to_string())
        .unwrap_or_else(|| "null".to_string());

    let idb_worker = rt.idb_worker.lock().unwrap().tx.clone();

    let cache_storage = CacheStorage {
        origin,
        idb_worker: Arc::new(idb_worker),
        cache_list: Arc::new(Mutex::new(HashMap::new())),
    };

    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            let cs = Class::instance(ctx.clone(), cache_storage)?;
            ctx.globals().set("caches", cs)?;

            // Also register Request and Response classes
            Class::<Request>::register(&ctx)?;
            Class::<Response>::register(&ctx)?;

            Ok(())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_operations() {
        // Mock test - would need actual runtime
        let origin = "http://example.com".to_string();

        let cache = Cache {
            name: "v1".to_string(),
            origin,
            idb_worker: Arc::new(|| {}),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        };

        assert_eq!(cache.name, "v1");
    }

    #[test]
    fn test_response_creation() {
        let response = Response {
            status: 200,
            status_text: "OK".to_string(),
            headers: HashMap::new(),
            body: vec![],
            url: "http://example.com".to_string(),
            redirected: false,
        };

        assert!(response.ok());
        assert_eq!(response.status, 200);
    }
}
