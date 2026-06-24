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
        let rt: JsRuntime = globals.get("__albedo_rt__").expect("Albedo Engine: internal invariant violated");

        let (id, sender) = {
            let mut el = rt.event_loop.lock().unwrap_or_else(|e| e.into_inner());
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
                            .expect("Albedo Engine: internal invariant violated")
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
