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
