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
            if let Err(e) = crate::js::bindings::document::register(&rt, dom.clone(), engine.stylesheet.clone(), engine.primitives.clone(), url.to_string(), "".to_string()) {
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
    {
        let ctx = rt.context.lock().unwrap();
        ctx.with(|ctx| {
             let navigator = ctx.globals().get::<_, rquickjs::Object>("navigator")?;
             let clipboard = rquickjs::Object::new(ctx.clone())?;
             clipboard.set("writeText", rquickjs::Function::new(ctx.clone(), |_: String| -> rquickjs::Result<()> { Ok(()) }))?;
             clipboard.set("readText", rquickjs::Function::new(ctx.clone(), |ctx: Ctx| -> rquickjs::Result<Value> { 
                 rquickjs::Promise::new(ctx, |resolve, _| { resolve.resolve("") }).map(|p| p.into_value())
             }))?;
             navigator.set("clipboard", clipboard)?;
             Ok::<_, rquickjs::Error>(())
        })?;
    }
    crate::js::bindings::location::register(&rt.context.lock().unwrap(), url, rt.pending_navigation.clone())?;
    crate::js::bindings::shims::register(&rt.context.lock().unwrap())?;
    
    {
        let ctx = rt.context.lock().unwrap();
        ctx.with(|ctx| {
             crate::js::bindings::url_search_params::register(&ctx)?;
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
            
            let get_selection = rquickjs::Function::new(ctx.clone(), |ctx: Ctx| -> rquickjs::Result<Value> {
                let selection = crate::js::bindings::selection::Selection::new();
                let instance = Class::instance(ctx, selection)?;
                Ok(instance.into_value())
            })?;
            global.set("getSelection", get_selection)?;

            // Viewport & Scale stubs
            global.set("innerWidth", 1280)?;
            global.set("innerHeight", 720)?;
            global.set("devicePixelRatio", 1.0)?;

            // History stub
            let history = ctx.globals().get::<_, rquickjs::Object>("Object")?
                .construct::<_, rquickjs::Object>(())?;
            history.set("back", rquickjs::Function::new(ctx.clone(), || {}))?;
            history.set("forward", rquickjs::Function::new(ctx.clone(), || {}))?;
            history.set("pushState", rquickjs::Function::new(ctx.clone(), |_: Value, _: String, _: Option<String>| {}))?;
            history.set("replaceState", rquickjs::Function::new(ctx.clone(), |_: Value, _: String, _: Option<String>| {}))?;
            global.set("history", history)?;

            // Animation Frame stubs
            let raf = rquickjs::Function::new(ctx.clone(), |ctx: Ctx, cb: Function| -> rquickjs::Result<i32> {
                // Simplified: just call it in the next "tick" (timeout 16ms)
                let cb_static: Function<'static> = unsafe { std::mem::transmute(cb) };
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(16));
                    // Note: This is an architectural shortcut. 
                    // In a real engine, this would be queued in the main loop.
                    let _ = cb_static.call::<_, ()>(());
                });
                Ok(1)
            })?;
            global.set("requestAnimationFrame", raf)?;
            global.set("cancelAnimationFrame", rquickjs::Function::new(ctx.clone(), |_: i32| {}))?;

            // Global Scroll & Viewport
            global.set("scrollX", 0.0)?;
            global.set("scrollY", 0.0)?;
            global.set("pageXOffset", 0.0)?;
            global.set("pageYOffset", 0.0)?;

            // Modern Observer stubs
            let intersection_obs = ctx.globals().get::<_, rquickjs::Object>("Object")?
                .construct::<_, rquickjs::Object>(())?;
            intersection_obs.set("prototype", ctx.globals().get::<_, rquickjs::Object>("Object")?.construct::<_, rquickjs::Object>(())?)?;
            global.set("IntersectionObserver", rquickjs::Function::new(ctx.clone(), |_: Function| {}))?;
            global.set("ResizeObserver", rquickjs::Function::new(ctx.clone(), |_: Function| {}))?;

            // Modern Web APIs stubs
            global.set("matchMedia", rquickjs::Function::new(ctx.clone(), |ctx: Ctx, _: String| -> rquickjs::Result<rquickjs::Object> {
                let mql = ctx.globals().get::<_, rquickjs::Object>("Object")?.construct::<_, rquickjs::Object>(())?;
                mql.set("matches", true)?;
                mql.set("media", "")?;
                mql.set("onchange", Value::new_null(ctx.clone()))?;
                mql.set("addListener", rquickjs::Function::new(ctx.clone(), || {}))?;
                mql.set("removeListener", rquickjs::Function::new(ctx.clone(), || {}))?;
                Ok(mql)
            }))?;

            global.set("scrollTo", rquickjs::Function::new(ctx.clone(), |_: f32, _: f32| {}))?;
            global.set("scrollBy", rquickjs::Function::new(ctx.clone(), |_: f32, _: f32| {}))?;
            
            global.set("alert", rquickjs::Function::new(ctx.clone(), |msg: String| {
                println!("[Albedo Alert] {}", msg);
            }))?;
            global.set("confirm", rquickjs::Function::new(ctx.clone(), |msg: String| -> bool {
                println!("[Albedo Confirm] {}", msg);
                true
            }))?;

            let custom_elements = ctx.globals().get::<_, rquickjs::Object>("Object")?
                .construct::<_, rquickjs::Object>(())?;
            custom_elements.set("define", rquickjs::Function::new(ctx.clone(), || {}))?;
            custom_elements.set("get", rquickjs::Function::new(ctx.clone(), || Value::new_null(ctx.clone())))?;
            custom_elements.set("whenDefined", rquickjs::Function::new(ctx.clone(), || Value::new_null(ctx.clone())))?;
            global.set("customElements", custom_elements)?;

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
