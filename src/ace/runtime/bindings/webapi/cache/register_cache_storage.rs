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
// PUBLIC API REGISTRATION
// ============================================================================

/// TODO: add docs
pub fn register_cache_storage(rt: &JsRuntime) -> JsResult<()> {
    let origin = rt
        .origin
        .lock().unwrap_or_else(|e| e.into_inner())
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
