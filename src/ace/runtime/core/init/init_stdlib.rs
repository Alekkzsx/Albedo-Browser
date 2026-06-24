use super::*;
use super::runtime::JsRuntime;
use crate::ace::engine::AceEngine;
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn init_stdlib(rt: &JsRuntime, url: &str) -> JsResult<()> {
    register_events(rt)?;
    crate::ace::runtime::bindings::utils::console::Console::register(rt)?;
    crate::ace::runtime::bindings::webapi::timers::register(rt)?;
    crate::ace::runtime::bindings::webapi::match_media::register(rt)?;
    crate::ace::runtime::bindings::webapi::fetch::register(rt)?;
    crate::ace::runtime::bindings::webapi::websocket::register(rt)?;
    crate::ace::runtime::bindings::webapi::worker::register(rt)?;
    crate::ace::runtime::bindings::webapi::indexeddb::register(rt)?;
    crate::ace::runtime::bindings::webapi::file_api::register(rt)?;
    crate::ace::runtime::bindings::webapi::crypto::register(rt)?;
    crate::ace::runtime::bindings::webapi::text_encoding::register(rt)?;
    crate::ace::runtime::bindings::webapi::structured_clone::register(rt)?;
    crate::ace::runtime::bindings::webapi::performance::register(rt)?;
    crate::ace::runtime::bindings::webapi::abort_controller::register(rt)?;

    // Service Worker, Cache, and Background Sync APIs (NEW)
    crate::ace::runtime::bindings::webapi::cache::register_cache_storage(rt)?;
    crate::ace::runtime::bindings::webapi::sync::register_sync_events(rt)?;
    crate::ace::runtime::bindings::webapi::service_worker_container::register_service_worker_container(
        rt,
    )?;

    // Register window proxy and post message using the context
    {
        let ctx = rt.context.lock().unwrap_or_else(|e| e.into_inner());
        ctx.with(|ctx_req| {
            let _ = crate::ace::runtime::bindings::webapi::window_proxy::register(&ctx_req);
            let _ = crate::ace::runtime::bindings::webapi::post_message::register(&ctx_req);
        });
    }
    crate::ace::runtime::bindings::webapi::notification::register(&rt.context.lock().unwrap_or_else(|e| e.into_inner()))?;
    crate::ace::runtime::bindings::webapi::geolocation::register(&rt.context.lock().unwrap_or_else(|e| e.into_inner()))?;
    crate::ace::runtime::bindings::webapi::url::register(rt)?;
    crate::ace::runtime::bindings::webapi::history::register(rt)?;
    crate::ace::runtime::bindings::webapi::navigator::register(&rt.context.lock().unwrap_or_else(|e| e.into_inner()))?;
    // `navigator` and `clipboard` bindings are registered by their modules.
    crate::ace::runtime::bindings::webapi::location::register(
        &rt.context.lock().unwrap_or_else(|e| e.into_inner()),
        url,
        rt.pending_navigation.clone(),
    )?;
    crate::ace::runtime::bindings::utils::shims::register(rt)?;

    {
        let ctx = rt.context.lock().unwrap_or_else(|e| e.into_inner());
        ctx.with(|ctx| {
            crate::ace::runtime::bindings::webapi::url_search_params::register(&ctx)?;
            Ok::<_, rquickjs::Error>(())
        })?;
    }

    // Window/Self alias (Circular global)
    {
        let ctx = rt.context.lock().unwrap_or_else(|e| e.into_inner());
        ctx.with(|ctx| {
            let global = ctx.globals();
            global.set("window", global.clone())?;
            global.set("self", global.clone())?;
            
            // Global event listeners, stubs and helpers implemented in JS to avoid lifetime issues
            ctx.eval::<(), _>(r#"
                globalThis._listeners = {};
                globalThis.addEventListener = function(type, listener) {
                    if (!globalThis._listeners[type]) {
                        globalThis._listeners[type] = [];
                    }
                    globalThis._listeners[type].push(listener);
                };
                globalThis.dispatchEvent = function(event) {
                    var type = event.type;
                    if (globalThis._listeners[type]) {
                        for (var i = 0; i < globalThis._listeners[type].length; i++) {
                            globalThis._listeners[type][i](event);
                        }
                    }
                    return true;
                };
                globalThis.getSelection = function() { return null; };
                globalThis.innerWidth = 1280;
                globalThis.innerHeight = 720;
                globalThis.devicePixelRatio = 1.0;
                globalThis.devicePixelRatio = 1.0;
                globalThis.scrollX = 0.0;
                globalThis.scrollY = 0.0;
                globalThis.pageXOffset = 0.0;
                globalThis.pageYOffset = 0.0;
                globalThis.scrollTo = function() {};
                globalThis.scrollBy = function() {};
                globalThis.alert = function(msg) { console.warn('ALERT:', msg); };
                globalThis.confirm = function(msg) { console.warn('CONFIRM:', msg); return true; };
                globalThis.customElements = { define: function(){}, get: function(){ return null; }, whenDefined: function(){ return null; } };
            "#)?;

            // Define observer classes
            rquickjs::Class::<crate::ace::runtime::bindings::html::mutation_observer::MutationObserver>::define(&global)?;
            rquickjs::Class::<crate::ace::runtime::bindings::html::resize_observer::ResizeObserver>::define(&global)?;
            rquickjs::Class::<crate::ace::runtime::bindings::html::intersection_observer::IntersectionObserver>::define(&global)?;

            Ok::<_, rquickjs::Error>(())
        })?;
    }

    Ok(())
}
