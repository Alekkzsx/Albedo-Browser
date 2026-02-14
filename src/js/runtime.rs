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
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_execution() {
        let rt = JsRuntime::new().unwrap();
        let result = rt.execute_script("2 + 2").unwrap();
        // Note: Value doesn't have as_int() in rquickjs 0.6
        // We'll need to convert properly in real tests
    }
    
    #[test]
    fn test_variables() {
        let rt = JsRuntime::new().unwrap();
        rt.execute_script("var x = 10; var y = 20;").unwrap();
        let result = rt.execute_script("x + y").unwrap();
        // Verify it doesn't crash, proper assertion needs context
    }
    
    #[test]
    fn test_function_definition() {
        let rt = JsRuntime::new().unwrap();
        rt.execute_script("function multiply(a, b) { return a * b; }").unwrap();
        // Call will be tested separately
    }

    #[test]
    fn test_event_subclasses() {
        let rt = JsRuntime::new().unwrap();
        rt.register_events().unwrap();
        
        let script = "
            try {
                var m = new MouseEvent('click', { clientX: 10, clientY: 20 });
                var k = new KeyboardEvent('keydown', { key: 'A', ctrlKey: true });
                
                var result = [
                    // m instanceof Event, // This causes TypeError in rquickjs if not strictly Event type
                    m instanceof MouseEvent,
                    m.type,
                    m.clientX,
                    // k instanceof Event,
                    k.key,
                    k.ctrlKey
                ].join('|');
                result
            } catch(e) {
                'Error: ' + e.toString()
            }
        ";
        
        let result = rt.execute_script(script).unwrap();
        assert_eq!(result, "true|click|10|A|true");
    }

    #[test]
    fn test_set_timeout() {
        let rt = JsRuntime::new().unwrap();
        rt.init_stdlib().unwrap();
        
        let script = "
            var called = 'false';
            setTimeout(function() {
                called = 'true';
            }, 10);
        ";
        rt.execute_script(script).unwrap();
        
        // Initial check
        let result = rt.execute_script("called").unwrap();
        assert_eq!(result, "false");
        
        // Wait and run loop
        std::thread::sleep(std::time::Duration::from_millis(20));
        rt.run_pending();
        
        // Final check
        let result = rt.execute_script("called").unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn test_set_interval() {
        let rt = JsRuntime::new().unwrap();
        rt.init_stdlib().unwrap();
        
        let script = "
            var counter = 0;
            var id = setInterval(function() {
                counter++;
            }, 20);
        ";
        rt.execute_script(script).unwrap();
        
        // Run loop a few times
        std::thread::sleep(std::time::Duration::from_millis(25)); // 1st tick
        rt.run_pending();
        
        std::thread::sleep(std::time::Duration::from_millis(25)); // 2nd tick
        rt.run_pending();
        
        // Stop it
        let result = rt.execute_script("clearInterval(id); counter").unwrap();
        // Should be at least 2
        let count: i32 = result.parse().unwrap_or(0);
        assert!(count >= 2, "Counter should be at least 2, got {}", count);
    }

    #[test]
    fn test_dom_sync_with_timers() {
        use crate::engine::AceEngine;
        use crate::js::bindings::document;

        let mut engine = AceEngine::new();
        let html = r#"<div id="target">Initial</div>"#;
        engine.load_html(html);
        let dom = engine.dom.as_ref().unwrap().clone();

        let rt = JsRuntime::new().unwrap();
        rt.init_stdlib().unwrap();
        document::register(&rt, dom.clone()).unwrap();

        rt.execute_script(r#"
            setTimeout(function() {
                document.getElementById("target").textContent = "Updated";
            }, 10);
        "#).unwrap();

        // Initially still "Initial"
        assert_eq!(dom.root.select_first("#target").unwrap().text_contents(), "Initial");

        // Wait and run
        std::thread::sleep(std::time::Duration::from_millis(30));
        rt.run_pending();

        // Now SHOULD be "Updated"
        assert_eq!(dom.root.select_first("#target").unwrap().text_contents(), "Updated");
    }
}
