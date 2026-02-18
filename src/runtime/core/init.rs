use super::runtime::JsRuntime;
use crate::engine::AceEngine;
use rquickjs::{Context, Runtime, Ctx, Value, Class, Function, Persistent, prelude::*};
use std::sync::{Arc, Mutex};

type JsResult<T> = Result<T, rquickjs::Error>;

pub fn init_js_for_url(url: &str, engine: &AceEngine) -> Option<JsRuntime> {
    if let Ok(mut rt) = JsRuntime::new() {
        // Note: set_userdata was removed from rquickjs API
        // let _ = rt.context.lock().unwrap().set_userdata(rt.clone());
        rt.context.lock().unwrap().with(|ctx| {
            let _ = ctx.globals().set("__albedo_rt__", rt.clone());
        });
        rt.resource_manager = engine.resource_manager.clone();
        rt.origin = crate::network::security::Origin::from_url(url);
        let origin_str = get_origin(url); 
        if let Err(e) = init_storage(&rt, origin_str) {
            eprintln!("Failed to initialize storage for {}: {}", origin_str, e);
        }
        
        if let Some(dom) = &engine.dom {
            if let Err(e) = crate::runtime::bindings::html::document::register(&rt, dom.clone(), engine.stylesheet.clone(), engine.primitives.clone(), engine.canvas_contexts.clone(), url.to_string(), "".to_string(), engine.resource_manager.clone()) {
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
    crate::runtime::bindings::webapi::fetch::register(rt)?;
    crate::runtime::bindings::webapi::websocket::register(rt)?;
    crate::runtime::bindings::webapi::worker::register(rt)?;
    crate::runtime::bindings::webapi::indexeddb::register(rt)?;
    crate::runtime::bindings::webapi::file_api::register(rt)?;
    crate::runtime::bindings::webapi::crypto::register(rt)?;
    crate::runtime::bindings::webapi::notification::register(&rt.context.lock().unwrap())?;
    crate::runtime::bindings::webapi::geolocation::register(&rt.context.lock().unwrap())?;
    crate::runtime::bindings::webapi::url::register(rt)?;
    crate::runtime::bindings::webapi::history::register(rt)?;
    crate::runtime::bindings::webapi::navigator::register(&rt.context.lock().unwrap())?;
    // `navigator` and `clipboard` bindings are registered by their modules.
    crate::runtime::bindings::webapi::location::register(&rt.context.lock().unwrap(), url, rt.pending_navigation.clone())?;
    crate::runtime::bindings::utils::shims::register(&rt.context.lock().unwrap())?;
    
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
                globalThis.addEventListener = function() {};
                globalThis.dispatchEvent = function() { return true; };
                globalThis.getSelection = function() { return null; };
                globalThis.innerWidth = 1280;
                globalThis.innerHeight = 720;
                globalThis.devicePixelRatio = 1.0;
                globalThis.devicePixelRatio = 1.0;
                globalThis.scrollX = 0.0;
                globalThis.scrollY = 0.0;
                globalThis.pageXOffset = 0.0;
                globalThis.pageYOffset = 0.0;
                globalThis.matchMedia = function() { return { matches: true, media: '', onchange: null, addListener: function(){}, removeListener: function(){} }; };
                globalThis.scrollTo = function() {};
                globalThis.scrollBy = function() {};
                globalThis.alert = function(msg) { /* stub */ };
                globalThis.confirm = function(msg) { return true; };
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
    let sanitized_origin = origin.replace("://", "_")
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
        use crate::runtime::bindings::html::event_subclasses::{MouseEvent, KeyboardEvent};
        rquickjs::Class::<MouseEvent>::define(&global)?;
        rquickjs::Class::<KeyboardEvent>::define(&global)?;
        
        // Setup prototype chain (basic inheritance simulation)
        
        let event_ctor: rquickjs::Function = global.get("Event")?;
        let mouse_ctor: rquickjs::Function = global.get("MouseEvent")?;
        let kbd_ctor: rquickjs::Function = global.get("KeyboardEvent")?;
        
        let event_proto: rquickjs::Object = event_ctor.get("prototype")?;
        let mouse_proto: rquickjs::Object = mouse_ctor.get("prototype")?;
        let kbd_proto: rquickjs::Object = kbd_ctor.get("prototype")?;
        
        mouse_proto.set_prototype(Some(&event_proto))?;
        kbd_proto.set_prototype(Some(&event_proto))?;
        
        Ok(())
    })
}
