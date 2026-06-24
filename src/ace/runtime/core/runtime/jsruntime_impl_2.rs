use super::*;
use crate::ace::engine::dom::AceDOM;
use crate::network::resources::ResourceManager;
use crate::shared::security::Origin;
use rquickjs::function::IntoJsFunc;
use rquickjs::{Context, Ctx, Runtime, Value};
use std::collections::HashMap;
use std::result::Result as StdResult;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};



use super::event_loop::EventLoop;

impl JsRuntime {

    /// TODO: add docs
    pub fn new() -> JsResult<Self> {
        let profiler = Arc::new(albedo_jit::JitProfiler::new(
            albedo_jit::ProfilerConfig::default(),
        ));
        let jit_bridge = Arc::new(albedo_jit::JitBridge::new(Arc::clone(&profiler)).expect("Albedo Engine: internal invariant violated"));
        let bytecode_registry = Arc::new(albedo_jit::BytecodeRegistry::new());
        Self::new_with_jit(jit_bridge, profiler, bytecode_registry)
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
        super::eval::execute_script(self, code)
    }

    /// Execute an ES Module inline
    pub fn execute_module(&self, code: &str, name: &str) -> JsResult<String> {
        super::eval::execute_module(self, code, name)
    }

    /// Load and execute an ES Module from URL
    pub fn load_module_from_url(&self, url: &str) -> JsResult<String> {
        super::eval::execute_module_from_url(self, url)
    }

    /*
    /// Call a JavaScript function by name
    pub fn call_function(&self, name: &str, args: Vec<Value<'static>>) -> JsResult<Value<'static>> {
        let ctx = self.context.lock().unwrap_or_else(|e| e.into_inner());
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
        let ctx = self.context.lock().unwrap_or_else(|e| e.into_inner());
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
        let ctx = self.context.lock().unwrap_or_else(|e| e.into_inner());
        f(&ctx)
    }

    /// TODO: add docs
    pub fn dispatch_event(&self, dom: Arc<Mutex<AceDOM>>, index: usize, type_: &str) {
        self.with_context(|ctx| {
            ctx.with(|ctx| {
                use crate::ace::runtime::bindings::html::element::Element;
                let element = Element {
                    dom,
                    index,
                    mutations: self.mutations.clone(),
                    stylesheet_dirty: self.stylesheet_dirty.clone(),
                    primitives: self.primitives.clone(),
                    canvas_contexts: self.canvas_contexts.clone(),
                    pending_scroll: self.pending_scroll.clone(),
                    element_geometry: self.element_geometry.clone(),
                    element_scroll: self.element_scroll.clone(),
                };
                if let Ok(instance) = rquickjs::Class::instance(ctx.clone(), element) {
                    let instance_val = instance.into_value();
                    let script = format!("new Event('{}', {{ bubbles: true }})", type_);
                    if let Ok(event_obj) = ctx.eval::<rquickjs::Value, _>(script) {
                        if let Some(obj) = instance_val.as_object() {
                            if let Ok(dispatch) = obj.get::<_, rquickjs::Function>("dispatchEvent")
                            {
                                let _: rquickjs::Result<rquickjs::Value> =
                                    dispatch.call((event_obj,));
                            }
                        }
                    }
                }
            })
        });
    }
}
