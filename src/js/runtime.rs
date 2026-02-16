use rquickjs::{Context, Runtime, Ctx, Value, Exception};
use rquickjs::function::IntoJsFunc;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::result::Result as StdResult;
use crate::engine::dom::AceDOM;

pub type JsResult<T> = StdResult<T, rquickjs::Error>;

#[derive(Clone, Default)]
pub struct HistoryEntry {
    pub url: String,
    pub state_json: Option<String>,
}

pub struct ResizeRegistry {
    pub observers: HashMap<usize, Vec<rquickjs::Persistent<rquickjs::Function<'static>>>>,
}

pub struct IntersectionRegistry {
    pub observers: HashMap<usize, Vec<(rquickjs::Persistent<rquickjs::Function<'static>>, f32)>>, // Simplified threshold
}

/// JavaScript runtime wrapper around QuickJS
/// 
///
/// Provides a safe, ergonomic interface for executing JavaScript code
/// and interacting with the JS environment.
pub struct JsRuntime {
    pub(crate) context: Arc<Mutex<Context>>,
    pub(crate) runtime: Arc<Mutex<Runtime>>,
    pub event_loop: Arc<Mutex<EventLoop>>,
    pub mutations: Arc<Mutex<bool>>,
    pub stylesheet_dirty: Arc<Mutex<bool>>,
    pub pending_navigation: Arc<Mutex<Option<String>>>,
    pub dom: Arc<Mutex<Option<Arc<Mutex<AceDOM>>>>>, // Link to Engine DOM
    pub primitives: Arc<Mutex<Vec<crate::engine::ACEPrimitive>>>,
    pub observer_registry: Arc<Mutex<HashMap<usize, rquickjs::Persistent<rquickjs::Function<'static>>>>>,
    pub history_stack: Arc<Mutex<Vec<HistoryEntry>>>,
    pub history_index: Arc<Mutex<usize>>,
    pub resize_registry: Arc<Mutex<HashMap<usize, Vec<rquickjs::Persistent<rquickjs::Function<'static>>>>>>,
    pub intersection_registry: Arc<Mutex<HashMap<usize, Vec<(rquickjs::Persistent<rquickjs::Function<'static>>, f32)>>>>,
    pub layout_states: Arc<Mutex<HashMap<usize, (f32, f32, f32, f32)>>>, // x, y, w, h
}

use crate::js::event_loop::EventLoop;

impl JsRuntime {
    /// Create a new JavaScript runtime with QuickJS
    pub fn new() -> JsResult<Self> {
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;
        
        let rt = Self {
            context: Arc::new(Mutex::new(context)),
            runtime: Arc::new(Mutex::new(runtime)),
            event_loop: Arc::new(Mutex::new(EventLoop::new())),
            mutations: Arc::new(Mutex::new(false)),
            stylesheet_dirty: Arc::new(Mutex::new(false)),
            pending_navigation: Arc::new(Mutex::new(None)),
            dom: Arc::new(Mutex::new(None)),
            primitives: Arc::new(Mutex::new(Vec::new())),
            observer_registry: Arc::new(Mutex::new(HashMap::new())),
            history_stack: Arc::new(Mutex::new(Vec::new())),
            history_index: Arc::new(Mutex::new(0)),
            resize_registry: Arc::new(Mutex::new(HashMap::new())),
            intersection_registry: Arc::new(Mutex::new(HashMap::new())),
            layout_states: Arc::new(Mutex::new(HashMap::new())),
        };

        // Store self in userdata for access from within JS callbacks
        rt.context.lock().unwrap().set_userdata(rt.clone());

        Ok(rt)
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
        crate::js::eval::execute_script(self, code)
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

    pub fn dispatch_event(&self, dom: Arc<Mutex<AceDOM>>, index: usize, type_: &str) {
        self.with_context(|ctx| {
            ctx.with(|ctx| {
                use crate::js::bindings::element::Element;
                let element = Element { 
                    dom,
                    index, 
                    mutations: self.mutations.clone(),
                    stylesheet_dirty: self.stylesheet_dirty.clone(),
                    primitives: self.primitives.clone(),
                };
                if let Ok(instance) = rquickjs::Class::instance(ctx.clone(), element) {
                    let instance_val = instance.into_value();
                    let script = format!("new Event('{}', {{ bubbles: true }})", type_);
                    if let Ok(event_obj) = ctx.eval::<rquickjs::Value, _>(script) {
                         if let Some(obj) = instance_val.as_object() {
                             if let Ok(dispatch) = obj.get::<_, rquickjs::Function>("dispatchEvent") {
                                 let _: rquickjs::Result<rquickjs::Value> = dispatch.call((event_obj,));
                             }
                         }
                    }
                }
            })
        });
    }


    pub fn get_pending_navigation(&self) -> Option<String> {
        let mut pending = self.pending_navigation.lock().unwrap();
        pending.take()
    }


    pub fn run_gc(&self) {
        let rt = self.runtime.lock().unwrap();
        rt.run_gc();
    }

    pub fn run_pending(&self) -> (bool, bool) {
        crate::js::executor::run_pending(self)
    }
}

impl Clone for JsRuntime {
    fn clone(&self) -> Self {
        Self {
            context: Arc::clone(&self.context),
            runtime: Arc::clone(&self.runtime),
            event_loop: Arc::clone(&self.event_loop),
            mutations: Arc::clone(&self.mutations),
            stylesheet_dirty: Arc::clone(&self.stylesheet_dirty),
            pending_navigation: Arc::clone(&self.pending_navigation),
            dom: Arc::clone(&self.dom),
            primitives: Arc::clone(&self.primitives),
            observer_registry: Arc::clone(&self.observer_registry),
            history_stack: Arc::clone(&self.history_stack),
            history_index: Arc::clone(&self.history_index),
            resize_registry: Arc::clone(&self.resize_registry),
            intersection_registry: Arc::clone(&self.intersection_registry),
            layout_states: Arc::clone(&self.layout_states),
        }
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
