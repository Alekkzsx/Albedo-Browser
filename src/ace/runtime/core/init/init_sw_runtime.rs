use super::*;
use super::runtime::JsRuntime;
use crate::ace::engine::AceEngine;
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn init_sw_runtime(_url: &str, origin: &str) -> Option<JsRuntime> {
    if let Ok(rt) = JsRuntime::new() {
        rt.context.lock().unwrap_or_else(|e| e.into_inner()).with(|ctx| {
            let _ = ctx.globals().set("__albedo_rt__", rt.clone());
        });
        *rt.origin.lock().unwrap_or_else(|e| e.into_inner()) = crate::network::security::Origin::from_url(origin);

        // Initialize basic stdlib for SW (subset of full stdlib)
        let _ = register_events(&rt);
        let _ = crate::ace::runtime::bindings::utils::console::Console::register(&rt);
        let _ = crate::ace::runtime::bindings::webapi::timers::register(&rt);
        let _ = crate::ace::runtime::bindings::webapi::fetch::register(&rt);
        let _ = crate::ace::runtime::bindings::webapi::indexeddb::register(&rt);
        let _ = crate::ace::runtime::bindings::webapi::crypto::register(&rt);
        let _ = crate::ace::runtime::bindings::webapi::url::register(&rt);
        let _ = crate::ace::runtime::bindings::webapi::text_encoding::register(&rt);
        let _ = crate::ace::runtime::bindings::webapi::structured_clone::register(&rt);

        // Circular global for ServiceWorkerGlobalScope
        rt.context.lock().unwrap_or_else(|e| e.into_inner()).with(|ctx| {
            let global = ctx.globals();
            let _ = global.set("self", global.clone());
            let _ = global.set("globalThis", global.clone());

            // SW specific Polyfills
            let _ = ctx.eval::<(), _>(
                r#"
                globalThis._listeners = {};
                globalThis.addEventListener = function(type, listener) {
                    if (!globalThis._listeners[type]) globalThis._listeners[type] = [];
                    globalThis._listeners[type].push(listener);
                };
                globalThis.dispatchEvent = function(event) {
                    var ls = globalThis._listeners[event.type];
                    if (ls) for (var i = 0; i < ls.length; i++) ls[i](event);
                    return true;
                };

                // Stub for registration objects
                if (!globalThis.registration) {
                    globalThis.registration = {
                        scope: '',
                        update: function() { return Promise.resolve(); },
                        unregister: function() { return Promise.resolve(true); },
                        showNotification: function() { return Promise.resolve(); }
                    };
                }

                // Global scope aliases
                globalThis.caches = globalThis.caches || {};
                globalThis.clients = globalThis.clients || {
                    claim: function() { return Promise.resolve(); },
                    matchAll: function() { return Promise.resolve([]); }
                };
            "#,
            );
        });

        // Register core SW bindings
        let _ = crate::ace::runtime::bindings::webapi::cache::register_cache_storage(&rt);
        let _ = crate::ace::runtime::bindings::webapi::sync::register_sync_events(&rt);
        let _ = crate::ace::runtime::bindings::webapi::service_worker_container::register_service_worker_container(&rt);

        return Some(rt);
    }
    None
}
