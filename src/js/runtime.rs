use rquickjs::{Context, Runtime, Ctx, Value};
use rquickjs::function::IntoJsFunc;
use std::sync::{Arc, Mutex};
use std::result::Result as StdResult;

pub type JsResult<T> = StdResult<T, rquickjs::Error>;

/// JavaScript runtime wrapper around QuickJS
/// 
/// Provides a safe, ergonomic interface for executing JavaScript code
/// and interacting with the JS environment.
pub struct JsRuntime {
    runtime: Arc<Mutex<Runtime>>,
    context: Arc<Mutex<Context>>,
    pub event_loop: Arc<Mutex<EventLoop>>,
}

use crate::js::event_loop::EventLoop;

impl JsRuntime {
    /// Create a new JavaScript runtime with QuickJS
    pub fn new() -> JsResult<Self> {
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;
        
        Ok(Self {
            runtime: Arc::new(Mutex::new(runtime)),
            context: Arc::new(Mutex::new(context)),
            event_loop: Arc::new(Mutex::new(EventLoop::new())),
        })
    }
    
    /// Execute a JavaScript code string
    /// 
    /// # Example
    /// ```ignore
    /// let mut rt = JsRuntime::new()?;
    /// let result = rt.execute_script("2 + 2")?;
    /// assert_eq!(result.as_int(), Some(4));
    /// ```
    pub fn execute_script(&self, code: &str) -> JsResult<String> {
        let ctx = self.context.lock().unwrap();
        ctx.with(|ctx| {
            let result: Value = ctx.eval(code)?;
            
            // Try to convert to string, fallback to debug
            if let Some(s) = result.as_string() {
                Ok(s.to_string()?)
            } else if result.is_null() {
                Ok("null".to_string())
            } else if result.is_undefined() {
                Ok("undefined".to_string())
            } else {
                // Try JSON stringify
                match ctx.json_stringify(result.clone()) {
                    Ok(Some(s)) => Ok(s.to_string()?),
                    _ => Ok(format!("{:?}", result))
                }
            }
        })
    }
    
    /*
    /// Call a JavaScript function by name
    pub fn call_function(&self, name: &str, args: Vec<Value<'static>>) -> JsResult<Value<'static>> {
        let ctx = self.context.lock().unwrap();
        ctx.with(|ctx: Ctx| {
            let global = ctx.globals();
            let func: Function = global.get(name)?;
            // Requires converting Vec to tuple or implementing IntoArgs, skipping for now
            // let result: Value = func.call(args)?; 
            // Ok(result.into_js(&ctx)?)
            Ok(Value::new_undefined(ctx.clone()).into_js(&ctx)?)
        })
    }
    */
    
    /// Register a Rust function in the global scope
    /// 
    /// # Example
    /// ```ignore
    /// rt.register_global_function("alert", |msg: String| {
    ///     println!("ALERT: {}", msg);
    /// })?;
    /// rt.execute_script("alert('Hello from JS!')")?;
    /// ```
    pub fn register_global_function<F, A>(&self, name: &str, func: F) -> JsResult<()>
    where
        F: for<'a> IntoJsFunc<'a, A> + 'static,
    {
        let ctx = self.context.lock().unwrap();
        ctx.with(|ctx: Ctx| {
            let global = ctx.globals();
            global.set(name, rquickjs::Function::new(ctx.clone(), func)?)?;
            Ok(())
        })
    }
    
    /// Get the QuickJS context for advanced operations
    pub(crate) fn with_context<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Context) -> R,
    {
        let ctx = self.context.lock().unwrap();
        f(&ctx)
    }


    pub fn init_stdlib(&self) -> JsResult<()> {
        let _ = self.register_events();
        let _ = crate::js::console::Console::register(self);
        let _ = crate::js::bindings::timers::register(self);
        let _ = crate::js::bindings::fetch::register(self);
        // localStorage/sessionStorage are initialized separately via init_storage per domain
        Ok(())
    }

    pub fn init_storage(&self, origin: &str) -> JsResult<()> {
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
        
        self.with_context(|ctx| {
            ctx.with(|ctx| {
                Storage::register(&ctx, "localStorage", Storage::new_local(local_storage_path))?;
                Storage::register(&ctx, "sessionStorage", Storage::new_session())?;
                Ok(())
            })
        })
    }

    pub fn register_events(&self) -> JsResult<()> {
        let ctx = self.context.lock().unwrap();
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
            // MouseEvent.prototype.__proto__ = Event.prototype
            // This allows 'instanceof Event' to work
            
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

    pub fn run_gc(&self) {
        let rt = self.runtime.lock().unwrap();
        rt.run_gc();
    }

    pub fn run_pending(&self) -> bool {
        let mut executed = false;
        
        // 1. Run QuickJS pending jobs (Promises/microtasks)
        {
            let ctx = self.context.lock().unwrap();
            ctx.with(|ctx| {
                if ctx.execute_pending_job() {
                    executed = true;
                }
            });
        }

        // 2. Run EventLoop tasks (timers, etc)
        let (timers, macros) = {
            let mut el = self.event_loop.lock().unwrap();
            el.take_pending_tasks()
        };
        
        if !timers.is_empty() {
             self.with_context(|ctx| {
                ctx.with(|ctx| {
                    for timer in timers {
                         if let Ok(func) = timer.callback.restore(&ctx) {
                            let _: rquickjs::Result<Value> = func.call(());
                            executed = true;
                        }
                    }
                    
                    // Run pending jobs AGAIN after timers might have resolved promises
                    if ctx.execute_pending_job() {
                        executed = true;
                    }
                })
             });
        }
             
        for task in macros {
            task();
            executed = true;
        }
        
        executed
    }
}

impl Clone for JsRuntime {
    fn clone(&self) -> Self {
        Self {
            runtime: Arc::clone(&self.runtime),
            context: Arc::clone(&self.context),
            event_loop: Arc::clone(&self.event_loop),
        }
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
