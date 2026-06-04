use crate::ace::runtime::core::runtime::JsRuntime;
use crate::ace::runtime::core::sw_db::{ServiceWorkerDatabase, SwCacheEntryData};
use rquickjs::{Class, Ctx, Object, Persistent, Result as JsResult, Value};
use crate::ace::json::{self, JsonValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// CACHE ENTRY STORAGE (for IndexedDB persistence)
// ============================================================================

#[derive(Clone, Debug)]
pub struct CacheEntry {
    pub url: String,
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub timestamp: u64,          // unix timestamp
    pub expires_at: Option<u64>, // unix timestamp or None = no expiry
}

#[derive(Clone, Debug)]
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
    pub db: Arc<ServiceWorkerDatabase>,
}

#[rquickjs::methods]
impl Cache {
    /// cache.add(url): Fetch from network and store in cache
    pub fn add<'js>(&self, ctx: Ctx<'js>, url: String) -> JsResult<Value<'js>> {
        let (promise, resolve, reject) = rquickjs::Promise::new(&ctx)?;

        // Use JsRuntime to register promise in event loop
        let globals = ctx.globals();
        let rt: JsRuntime = globals.get("__albedo_rt__").unwrap();

        let (id, sender) = {
            let mut el = rt.event_loop.lock().unwrap();
            let id = el.register_promise(
                Persistent::save(&ctx, resolve),
                Persistent::save(&ctx, reject),
            );
            (id, el.async_sender.clone())
        };

        let db = self.db.clone();
        let cache_name = self.name.clone();
        let origin = self.origin.clone();
        let url_clone = url.clone();

        tokio::spawn(async move {
            let client = crate::network::client::FetchClient::new();
            match client.fetch(
                &url_clone,
                None,
                crate::network::security::Origin::from_url(&origin),
            ) {
                Ok(resp) => {
                    let entry = SwCacheEntryData {
                        id: crate::utils::uuid::Uuid::new_v4().to_string(),
                        cache_name,
                        origin,
                        url: url_clone,
                        status: resp.status,
                        headers: {
                            let mut h_map = HashMap::new();
                            for (k, v) in &resp.headers {
                                h_map.insert(k.clone(), JsonValue::String(v.clone()));
                            }
                            json::stringify(&JsonValue::Object(h_map))
                        },
                        body: resp.body_bytes,
                        created_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    };
                    if let Ok(_) = db.save_cache_entry(&entry) {
                        let _ = sender.send(crate::ace::runtime::core::event_loop::AsyncResult {
                            id,
                            result: Ok((200, "".to_string())), // Resolve void with 200 OK
                        });
                    } else {
                        let _ = sender.send(crate::ace::runtime::core::event_loop::AsyncResult {
                            id,
                            result: Err("Database error".to_string()),
                        });
                    }
                }
                Err(e) => {
                    let _ = sender.send(crate::ace::runtime::core::event_loop::AsyncResult {
                        id,
                        result: Err(e.to_string()),
                    });
                }
            }
        });

        Ok(promise.into_value())
    }

    /// cache.match(request, options): Look up request in cache
    pub fn match_url<'js>(
        &self,
        ctx: Ctx<'js>,
        url: String,
        _options: rquickjs::prelude::Opt<Object<'js>>,
    ) -> JsResult<Value<'js>> {
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;

        if let Ok(Some(entry)) = self.db.get_cache_entry(&self.name, &url) {
            let resp = Response {
                status: entry.status,
                status_text: "OK".to_string(),
                headers: {
                    if let Ok(JsonValue::Object(map)) = json::parse(&entry.headers) {
                        let mut h = HashMap::new();
                        for (k, v) in map {
                            if let Some(s) = v.as_string() {
                                h.insert(k, s.to_string());
                            }
                        }
                        h
                    } else {
                        HashMap::new()
                    }
                },
                body: entry.body,
                url,
                redirected: false,
            };
            let _ = resolve.call::<(Response,), ()>((resp,));
        } else {
            let _ = resolve.call::<(Option<Response>,), ()>((None,));
        }

        Ok(promise.into_value())
    }

    /// cache.delete(request): Delete entry from cache
    pub fn delete<'js>(&self, ctx: Ctx<'js>, url: String) -> JsResult<Value<'js>> {
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ = self.db.delete_cache_entry(&self.name, &url);
        let _ = resolve.call::<(bool,), ()>((true,));
        Ok(promise.into_value())
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
    pub db: Arc<ServiceWorkerDatabase>,
    #[qjs(skip_trace)]
    pub cache_list: Arc<Mutex<HashMap<String, Cache>>>,
}

#[rquickjs::methods]
impl CacheStorage {
    /// caches.open(name): Open/create cache with given name
    pub fn open<'js>(&self, ctx: Ctx<'js>, name: String) -> JsResult<Value<'js>> {
        // Check if already in memory
        let mut cache_list = self.cache_list.lock().map_err(|_| {
            rquickjs::Error::new_from_js("CacheStorage", "Failed to lock cache list")
        })?;

        if let Some(cache) = cache_list.get(&name) {
            let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
            let _ = resolve.call::<(Cache,), ()>((cache.clone(),));
            return Ok(promise.into_value());
        }

        // Create new cache
        let cache = Cache {
            name: name.clone(),
            origin: self.origin.clone(),
            db: self.db.clone(),
        };

        if let Ok(_) = self.db.save_cache(name.clone(), self.origin.clone()) {
            cache_list.insert(name.clone(), cache.clone());
        }

        // Return resolved Promise<Cache>
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ = resolve.call::<(Cache,), ()>((cache,));

        Ok(promise.into_value())
    }

    /// caches.keys(): List all cache names
    pub fn keys<'js>(&self, ctx: Ctx<'js>) -> JsResult<Value<'js>> {
        let cache_list = self.cache_list.lock().map_err(|_| {
            rquickjs::Error::new_from_js("CacheStorage", "Failed to lock cache list")
        })?;

        let arr = rquickjs::Array::new(ctx.clone())?;
        for (i, name) in cache_list.keys().enumerate() {
            arr.set(i, name.clone())?;
        }

        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ =
            resolve.call::<(Vec<String>,), ()>((cache_list.keys().cloned().collect::<Vec<_>>(),));

        Ok(promise.into_value())
    }

    /// caches.delete(name): Delete entire cache
    pub fn delete<'js>(&self, ctx: Ctx<'js>, name: String) -> JsResult<Value<'js>> {
        let mut cache_list = self.cache_list.lock().map_err(|_| {
            rquickjs::Error::new_from_js("CacheStorage", "Failed to lock cache list")
        })?;

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
        // Otimização: QuickJS já tem um parser de JSON nativo rápido.
        // Passamos os bytes diretamente para o motor JS.
        ctx.json_parse(self.body.clone())
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

    let sw_db = rt.sw_manager.db.clone();

    let cache_storage = CacheStorage {
        origin,
        db: sw_db,
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
            db: Arc::new(ServiceWorkerDatabase::new(std::path::PathBuf::from(":memory:")).unwrap()),
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
