use super::runtime::JsRuntime;
use crate::engine::AceEngine;
use std::sync::{Arc, Mutex};

type JsResult<T> = Result<T, rquickjs::Error>;

pub fn init_js_for_url(url: &str, engine: &AceEngine) -> Option<JsRuntime> {
    if let Ok(mut rt) = JsRuntime::new() {
        // ... (existing code omitted for brevity in thought, but I will provide the full block)
        // Actually I should provide the full block as per rules.
        // Wait, I'm using replace_file_content, so I need to match the block.
        // I'll use a smaller range.
        println!("[init_js_for_url] START for url={}", url);
        rt.context.lock().unwrap().with(|ctx| {
            let _ = ctx.globals().set("__albedo_rt__", rt.clone());
        });
        println!("[init_js_for_url] __albedo_rt__ set");
        *rt.resource_manager.lock().unwrap() = engine.resource_manager.clone();
        rt.element_geometry = engine.element_geometry.clone();
        rt.element_scroll = engine.element_scroll.clone();
        *rt.origin.lock().unwrap() = crate::network::security::Origin::from_url(url);
        let origin_str = get_origin(url);

        // ES Modules: Configurar ModuleRegistry e registrar Loader/Resolver
        {
            let mut registry = rt.module_registry.lock().unwrap();
            *registry = crate::runtime::core::module_loader::ModuleRegistry::new(url);
        }

        // Registrar o resolver e loader no Runtime do QuickJS
        {
            let registry_clone = rt.module_registry.lock().unwrap().clone();
            let resolver = crate::runtime::core::module_loader::AlbedoModuleResolver {
                registry: registry_clone.clone(),
            };
            let loader = crate::runtime::core::module_loader::AlbedoModuleLoader {
                registry: registry_clone,
            };
            let runtime = rt.runtime.lock().unwrap();
            runtime.set_loader(resolver, loader);
        }

        if let Err(e) = init_storage(&rt, origin_str) {
            eprintln!("Failed to initialize storage for {}: {}", origin_str, e);
        }
        crate::runtime::core::registry::register_runtime(rt.id, Arc::new(Mutex::new(rt.clone())));
        if let Some(dom) = &engine.dom {
            if let Err(e) = crate::runtime::bindings::html::document::register(
                &rt,
                dom.clone(),
                engine.stylesheet.clone(),
                engine.primitives.clone(),
                engine.canvas_contexts.clone(),
                url.to_string(),
                "".to_string(),
                engine.resource_manager.clone(),
            ) {
                eprintln!("Failed to register document API: {}", e);
            }
        }
        if let Err(e) = crate::runtime::bindings::utils::console::Console::register(&rt) {
            eprintln!("Failed to register console: {}", e);
        }
        if let Err(e) = register_events(&rt) {
            eprintln!("Failed to register events: {}", e);
        }
        if let Err(e) = init_stdlib(&rt, &url) {
            eprintln!("Failed to init stdlib: {}", e);
        }
        return Some(rt);
    }
    None
}

pub fn init_sw_runtime(_url: &str, origin: &str) -> Option<JsRuntime> {
    if let Ok(rt) = JsRuntime::new() {
        rt.context.lock().unwrap().with(|ctx| {
            let _ = ctx.globals().set("__albedo_rt__", rt.clone());
        });
        *rt.origin.lock().unwrap() = crate::network::security::Origin::from_url(origin);

        // Initialize basic stdlib for SW (subset of full stdlib)
        let _ = register_events(&rt);
        let _ = crate::runtime::bindings::utils::console::Console::register(&rt);
        let _ = crate::runtime::bindings::webapi::timers::register(&rt);
        let _ = crate::runtime::bindings::webapi::fetch::register(&rt);
        let _ = crate::runtime::bindings::webapi::indexeddb::register(&rt);
        let _ = crate::runtime::bindings::webapi::crypto::register(&rt);
        let _ = crate::runtime::bindings::webapi::url::register(&rt);
        let _ = crate::runtime::bindings::webapi::text_encoding::register(&rt);
        let _ = crate::runtime::bindings::webapi::structured_clone::register(&rt);

        // Circular global for ServiceWorkerGlobalScope
        rt.context.lock().unwrap().with(|ctx| {
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
        let _ = crate::runtime::bindings::webapi::cache::register_cache_storage(&rt);
        let _ = crate::runtime::bindings::webapi::sync::register_sync_events(&rt);
        let _ = crate::runtime::bindings::webapi::service_worker_container::register_service_worker_container(&rt);

        return Some(rt);
    }
    None
}

fn get_origin(url: &str) -> &str {
    if let Some(pos) = url.find("://") {
        let rest = &url[pos + 3..];
        if let Some(slash_pos) = rest.find('/') {
            return &url[..pos + 3 + slash_pos];
        } else {
            return url;
        }
    }
    url
}

pub fn init_stdlib(rt: &JsRuntime, url: &str) -> JsResult<()> {
    register_events(rt)?;
    crate::runtime::bindings::utils::console::Console::register(rt)?;
    crate::runtime::bindings::webapi::timers::register(rt)?;
    crate::runtime::bindings::webapi::match_media::register(rt)?;
    crate::runtime::bindings::webapi::fetch::register(rt)?;
    crate::runtime::bindings::webapi::websocket::register(rt)?;
    crate::runtime::bindings::webapi::worker::register(rt)?;
    crate::runtime::bindings::webapi::indexeddb::register(rt)?;
    crate::runtime::bindings::webapi::file_api::register(rt)?;
    crate::runtime::bindings::webapi::crypto::register(rt)?;
    crate::runtime::bindings::webapi::text_encoding::register(rt)?;
    crate::runtime::bindings::webapi::structured_clone::register(rt)?;

    // Service Worker, Cache, and Background Sync APIs (NEW)
    crate::runtime::bindings::webapi::cache::register_cache_storage(rt)?;
    crate::runtime::bindings::webapi::sync::register_sync_events(rt)?;
    crate::runtime::bindings::webapi::service_worker_container::register_service_worker_container(
        rt,
    )?;

    // Register window proxy and post message using the context
    {
        let ctx = rt.context.lock().unwrap();
        ctx.with(|ctx_req| {
            let _ = crate::runtime::bindings::webapi::window_proxy::register(&ctx_req);
            let _ = crate::runtime::bindings::webapi::post_message::register(&ctx_req);
        });
    }
    crate::runtime::bindings::webapi::notification::register(&rt.context.lock().unwrap())?;
    crate::runtime::bindings::webapi::geolocation::register(&rt.context.lock().unwrap())?;
    crate::runtime::bindings::webapi::url::register(rt)?;
    crate::runtime::bindings::webapi::history::register(rt)?;
    crate::runtime::bindings::webapi::navigator::register(&rt.context.lock().unwrap())?;
    // `navigator` and `clipboard` bindings are registered by their modules.
    crate::runtime::bindings::webapi::location::register(
        &rt.context.lock().unwrap(),
        url,
        rt.pending_navigation.clone(),
    )?;
    crate::runtime::bindings::utils::shims::register(rt)?;

    {
        let ctx = rt.context.lock().unwrap();
        ctx.with(|ctx| {
            crate::runtime::bindings::webapi::url_search_params::register(&ctx)?;
            Ok::<_, rquickjs::Error>(())
        })?;
    }

    // Window/Self alias (Circular global)
    {
        let ctx = rt.context.lock().unwrap();
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
            rquickjs::Class::<crate::runtime::bindings::html::mutation_observer::MutationObserver>::define(&global)?;
            rquickjs::Class::<crate::runtime::bindings::html::resize_observer::ResizeObserver>::define(&global)?;
            rquickjs::Class::<crate::runtime::bindings::html::intersection_observer::IntersectionObserver>::define(&global)?;

            Ok::<_, rquickjs::Error>(())
        })?;
    }

    Ok(())
}

pub fn init_storage(rt: &JsRuntime, origin: &str) -> JsResult<()> {
    let storage_dir = if let Ok(home) = std::env::var("HOME") {
        std::path::PathBuf::from(home).join(".local/share/albedo/storage")
    } else {
        std::path::PathBuf::from("./storage")
    };

    // Sanitize origin for filename
    let sanitized_origin = origin
        .replace("://", "_")
        .replace(".", "_")
        .replace("/", "_")
        .replace(":", "_");

    let local_storage_path = storage_dir.join(format!("{}.json", sanitized_origin));

    use crate::runtime::bindings::webapi::storage::Storage;

    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            Storage::register(&ctx, "localStorage", Storage::new_local(local_storage_path))?;
            Storage::register(&ctx, "sessionStorage", Storage::new_session())?;
            Ok(())
        })
    })
}

pub fn register_events(rt: &JsRuntime) -> JsResult<()> {
    let ctx = rt.context.lock().unwrap();
    ctx.with(|ctx: rquickjs::Ctx| {
        let global = ctx.globals();

        // Register base Event
        use crate::runtime::bindings::html::event::Event;
        rquickjs::Class::<Event>::define(&global)?;

        // Register subclasses
        use crate::runtime::bindings::html::event_subclasses::{
            KeyboardEvent, MessageEvent, MouseEvent, PointerEvent, TouchEvent,
        };
        rquickjs::Class::<MouseEvent>::define(&global)?;
        rquickjs::Class::<PointerEvent>::define(&global)?;
        rquickjs::Class::<TouchEvent>::define(&global)?;
        rquickjs::Class::<KeyboardEvent>::define(&global)?;
        rquickjs::Class::<MessageEvent>::define(&global)?;

        // Setup prototype chain (basic inheritance simulation)

        let event_ctor: rquickjs::Function = global.get("Event")?;
        let mouse_ctor: rquickjs::Function = global.get("MouseEvent")?;
        let pointer_ctor: rquickjs::Function = global.get("PointerEvent")?;
        let touch_ctor: rquickjs::Function = global.get("TouchEvent")?;
        let kbd_ctor: rquickjs::Function = global.get("KeyboardEvent")?;

        let event_proto: rquickjs::Object = event_ctor.get("prototype")?;
        let mouse_proto: rquickjs::Object = mouse_ctor.get("prototype")?;
        let pointer_proto: rquickjs::Object = pointer_ctor.get("prototype")?;
        let touch_proto: rquickjs::Object = touch_ctor.get("prototype")?;
        let kbd_proto: rquickjs::Object = kbd_ctor.get("prototype")?;
        let msg_ctor: rquickjs::Function = global.get("MessageEvent")?;
        let msg_proto: rquickjs::Object = msg_ctor.get("prototype")?;

        mouse_proto.set_prototype(Some(&event_proto))?;
        pointer_proto.set_prototype(Some(&mouse_proto))?; // PointerEvent herda de MouseEvent (que herda de Event)
        touch_proto.set_prototype(Some(&event_proto))?;
        kbd_proto.set_prototype(Some(&event_proto))?;
        msg_proto.set_prototype(Some(&event_proto))?;

        Ok(())
    })
}
