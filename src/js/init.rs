use crate::js::JsRuntime;
use crate::engine::AceEngine;
use rquickjs::{Context, Runtime, Ctx, Value};
use std::sync::{Arc, Mutex};

type JsResult<T> = Result<T, rquickjs::Error>;

pub fn init_js_for_url(url: &str, engine: &AceEngine) -> Option<JsRuntime> {
    if let Ok(rt) = JsRuntime::new() {
        let origin = get_origin(url); 
        if let Err(e) = init_storage(&rt, origin) {
            eprintln!("Failed to initialize storage for {}: {}", origin, e);
        }
        
        if let Some(dom) = &engine.dom {
            if let Err(e) = crate::js::bindings::document::register(&rt, dom.clone(), engine.stylesheet.clone()) {
                    eprintln!("Failed to register document API: {}", e);
            }
        }
        if let Err(e) = crate::js::console::Console::register(&rt) {
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
    crate::js::console::Console::register(rt)?;
    crate::js::bindings::timers::register(rt)?;
    crate::js::bindings::fetch::register(rt)?;
    crate::js::bindings::navigator::register(&rt.context.lock().unwrap())?;
    crate::js::bindings::location::register(&rt.context.lock().unwrap(), url, rt.pending_navigation.clone())?;
    crate::js::bindings::shims::register(&rt.context.lock().unwrap())?;
    
    // Window/Self alias (Circular global)
    {
        let ctx = rt.context.lock().unwrap();
        ctx.with(|ctx| {
            let global = ctx.globals();
            global.set("window", global.clone())?;
            global.set("self", global.clone())?;
            
            // Global event listeners (window.addEventListener)
            use crate::js::bindings::event::EventTargetImpl;
            let add_event_listener = rquickjs::Function::new(ctx.clone(), |type_: String, listener: rquickjs::Function| {
                // Use a special pointer for window (usize::MAX)
                unsafe {
                    let listener_static: rquickjs::Function<'static> = std::mem::transmute(listener);
                    EventTargetImpl::add_listener(usize::MAX, type_, listener_static);
                }
            })?;
            global.set("addEventListener", add_event_listener)?;
            
            let dispatch_event = rquickjs::Function::new(ctx.clone(), |event: Value| -> bool {
                if let Some(obj) = event.as_object() {
                    if let Ok(type_val) = obj.get::<_, String>("type") {
                        let listeners_static = EventTargetImpl::get_listeners(usize::MAX, &type_val);
                        for listener_static in listeners_static {
                            let listener: rquickjs::Function = unsafe { std::mem::transmute(listener_static) };
                            let _: rquickjs::Result<Value> = listener.call((event.clone(),));
                        }
                    }
                }
                true
            })?;
            global.set("dispatchEvent", dispatch_event)?;

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
    
    use crate::js::bindings::storage::Storage;
    
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
        use crate::js::bindings::event::Event;
        rquickjs::Class::<Event>::define(&global)?;
        
        // Register subclasses
        use crate::js::bindings::event_subclasses::{MouseEvent, KeyboardEvent};
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
