use crate::ace::engine::dom::AceDOM;
use crate::network::resources::ResourceManager;
use crate::shared::security::Origin;
use rquickjs::function::IntoJsFunc;
use rquickjs::{Context, Ctx, Runtime, Value};
use std::collections::HashMap;
use std::result::Result as StdResult;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub type JsResult<T> = StdResult<T, rquickjs::Error>;

static NEXT_RUNTIME_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone, Default)]
pub struct HistoryEntry {
    pub url: String,
    pub referrer: String,
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
#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct JsRuntime {
    #[qjs(skip_trace)]
    pub id: usize,
    #[qjs(skip_trace)]
    pub(crate) context: Arc<Mutex<Context>>,
    #[qjs(skip_trace)]
    pub(crate) runtime: Arc<Mutex<Runtime>>,
    #[qjs(skip_trace)]
    pub event_loop: Arc<Mutex<EventLoop>>,
    #[qjs(skip_trace)]
    pub mutations: Arc<Mutex<bool>>,
    #[qjs(skip_trace)]
    pub stylesheet_dirty: Arc<Mutex<bool>>,
    #[qjs(skip_trace)]
    pub pending_navigation: Arc<Mutex<Option<String>>>,
    #[qjs(skip_trace)]
    pub dom: Arc<Mutex<Option<Arc<Mutex<AceDOM>>>>>, // Link to Engine DOM
    #[qjs(skip_trace)]
    pub primitives: Arc<Mutex<Vec<crate::ace::engine::ACEPrimitive>>>,
    #[qjs(skip_trace)]
    pub observer_registry:
        Arc<Mutex<HashMap<usize, rquickjs::Persistent<rquickjs::Function<'static>>>>>,
    #[qjs(skip_trace)]
    pub history_stack: Arc<Mutex<Vec<HistoryEntry>>>,
    #[qjs(skip_trace)]
    pub history_index: Arc<Mutex<usize>>,
    #[qjs(skip_trace)]
    pub resize_registry:
        Arc<Mutex<HashMap<usize, Vec<rquickjs::Persistent<rquickjs::Function<'static>>>>>>,
    #[qjs(skip_trace)]
    pub intersection_registry:
        Arc<Mutex<HashMap<usize, Vec<(rquickjs::Persistent<rquickjs::Function<'static>>, f32)>>>>,
    #[qjs(skip_trace)]
    pub layout_states: Arc<Mutex<HashMap<usize, (f32, f32, f32, f32)>>>, // x, y, w, h
    #[qjs(skip_trace)]
    pub canvas_contexts:
        Arc<Mutex<std::collections::HashMap<usize, crate::ace::engine::graphics::canvas2d::Canvas2D>>>,
    #[qjs(skip_trace)]
    pub pending_scroll: Arc<Mutex<Option<usize>>>,
    #[qjs(skip_trace)]
    pub resource_manager: Arc<Mutex<Option<ResourceManager>>>,
    #[qjs(skip_trace)]
    pub origin: Arc<Mutex<Option<Origin>>>,
    #[qjs(skip_trace)]
    pub element_geometry: Arc<Mutex<HashMap<usize, crate::ace::engine::ElementGeometry>>>,
    #[qjs(skip_trace)]
    pub element_scroll: Arc<Mutex<HashMap<usize, (f32, f32)>>>,
    #[qjs(skip_trace)]
    pub viewport_y: Arc<Mutex<f32>>,
    #[qjs(skip_trace)]
    /// Geometrias de elementos em subframes projetadas para coordenadas globais.
    /// Populado pelo AceEngine via collect_subframe_geometries() após cada layout.
    /// Chave: (iframe_node_idx * 1_000_000) + elem_node_idx
    pub iframe_projected_geometry: Arc<Mutex<HashMap<u64, crate::ace::engine::ElementGeometry>>>,
    #[qjs(skip_trace)]
    pub module_registry: Arc<Mutex<crate::ace::runtime::core::module_loader::ModuleRegistry>>,
    #[qjs(skip_trace)]
    pub import_map: Arc<Mutex<Option<crate::ace::runtime::core::module_loader::ImportMap>>>,
    #[qjs(skip_trace)]
    pub screen_size: Arc<Mutex<(i32, i32)>>,
    #[qjs(skip_trace)]
    pub idb_worker:
        Arc<Mutex<crate::ace::runtime::bindings::webapi::idb_service::worker::IDBServiceWorker>>,
    #[qjs(skip_trace)]
    pub mql_registry: Arc<Mutex<Vec<crate::ace::runtime::bindings::webapi::match_media::MqlEntry>>>,
    #[qjs(skip_trace)]
    pub sw_manager: Arc<crate::ace::runtime::core::service_worker::ServiceWorkerManager>,
    #[qjs(skip_trace)]
    pub profiler: Arc<albedo_jit::JitProfiler>,
    #[qjs(skip_trace)]
    pub jit_bridge: Arc<albedo_jit::JitBridge>,
    #[qjs(skip_trace)]
    pub bytecode_registry: Arc<albedo_jit::BytecodeRegistry>,
    #[qjs(skip_trace)]
    pub interceptor: Arc<crate::ace::runtime::bridge::quickjs_intercept::QuickJsInterceptor>,
    #[qjs(skip_trace)]
    pub ready_state: Arc<Mutex<String>>,
    #[qjs(skip_trace)]
    pub page_start_time: std::time::Instant,
}

use super::event_loop::EventLoop;

impl JsRuntime {
    /// Create a new JavaScript runtime with QuickJS
    pub fn new_with_jit(
        jit_bridge: Arc<albedo_jit::JitBridge>,
        profiler: Arc<albedo_jit::JitProfiler>,
        bytecode_registry: Arc<albedo_jit::BytecodeRegistry>,
    ) -> JsResult<Self> {
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;
        let id = NEXT_RUNTIME_ID.fetch_add(1, Ordering::SeqCst);

        let event_loop = EventLoop::new();
        let idb_worker =
            crate::ace::runtime::bindings::webapi::idb_service::worker::IDBServiceWorker::new(
                event_loop.idb_sender.clone(),
            );

        let sw_db = Arc::new(
            crate::ace::runtime::core::sw_db::ServiceWorkerDatabase::new(std::path::PathBuf::from(
                "sw.db",
            ))
            .unwrap(),
        );
        let sw_manager =
            Arc::new(crate::ace::runtime::core::service_worker::ServiceWorkerManager::new(sw_db));

        let interceptor = Arc::new(
            crate::ace::runtime::bridge::quickjs_intercept::QuickJsInterceptor::new(
                Arc::clone(&jit_bridge),
                Arc::clone(&profiler),
                Arc::clone(&bytecode_registry),
            ),
        );

        let rt = Self {
            id,
            context: Arc::new(Mutex::new(context)),
            runtime: Arc::new(Mutex::new(runtime)),
            event_loop: Arc::new(Mutex::new(event_loop)),
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
            canvas_contexts: Arc::new(Mutex::new(HashMap::new())),
            pending_scroll: Arc::new(Mutex::new(None)),
            resource_manager: Arc::new(Mutex::new(None)),
            origin: Arc::new(Mutex::new(None)),
            element_geometry: Arc::new(Mutex::new(HashMap::new())),
            element_scroll: Arc::new(Mutex::new(HashMap::new())),
            viewport_y: Arc::new(Mutex::new(0.0)),
            iframe_projected_geometry: Arc::new(Mutex::new(HashMap::new())),
            module_registry: Arc::new(Mutex::new(
                crate::ace::runtime::core::module_loader::ModuleRegistry::new(""),
            )),
            import_map: Arc::new(Mutex::new(None)),
            screen_size: Arc::new(Mutex::new((1920, 1080))),
            idb_worker: Arc::new(Mutex::new(idb_worker)),
            mql_registry: Arc::new(Mutex::new(Vec::new())),
            sw_manager,
            profiler,
            jit_bridge,
            bytecode_registry,
            interceptor,
            ready_state: Arc::new(Mutex::new("loading".to_string())),
            page_start_time: std::time::Instant::now(),
        };

        // Configure OSR Interrupt Handler
        {
            let interceptor_clone = Arc::clone(&rt.interceptor);
            let runtime = rt.runtime.lock().unwrap();
            runtime.set_interrupt_handler(Some(Box::new(move || {
                interceptor_clone.handle_interrupt_no_ctx()
            })));
        }

        Ok(rt)
    }

    pub fn set_ready_state(&self, state: &str) {
        if let Ok(mut rs) = self.ready_state.lock() {
            if *rs == state {
                return;
            }
            *rs = state.to_string();
        }

        // Dispatch events based on state transition
        if state == "interactive" {
            let _ = self.execute_script("globalThis.document.dispatchEvent(new Event('DOMContentLoaded', { bubbles: true, cancelable: true }));");
        } else if state == "complete" {
            let _ = self.execute_script("globalThis.dispatchEvent(new Event('load', { bubbles: true, cancelable: true }));");
        }
    }

    pub fn new() -> JsResult<Self> {
        let profiler = Arc::new(albedo_jit::JitProfiler::new(
            albedo_jit::ProfilerConfig::default(),
        ));
        let jit_bridge = Arc::new(albedo_jit::JitBridge::new(Arc::clone(&profiler)).unwrap());
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

    pub fn dispatch_keyboard_event(
        &self,
        index: usize,
        type_: &str,
        key: &str,
        code: &str,
        ctrl: bool,
        shift: bool,
        alt: bool,
        meta: bool,
    ) {
        if let Some(ref dom_arc) = *self.dom.lock().unwrap() {
            let dom = dom_arc.clone();
            self.with_context(|ctx| {
                ctx.with(|ctx| {
                    use crate::ace::runtime::bindings::html::element::Element;
                    use crate::ace::runtime::bindings::html::event_subclasses::KeyboardEvent;

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

                        // Create KeyboardEvent options object
                        let opts = rquickjs::Object::new(ctx.clone()).unwrap();
                        let _ = opts.set("key", key);
                        let _ = opts.set("code", code);
                        let _ = opts.set("ctrlKey", ctrl);
                        let _ = opts.set("shiftKey", shift);
                        let _ = opts.set("altKey", alt);
                        let _ = opts.set("metaKey", meta);
                        let _ = opts.set("bubbles", true);

                        if let Ok(kb_event) = rquickjs::Class::instance(
                            ctx.clone(),
                            KeyboardEvent::new(type_.to_string(), Some(opts.into_value())),
                        ) {
                            if let Some(obj) = instance_val.as_object() {
                                if let Ok(dispatch) =
                                    obj.get::<_, rquickjs::Function>("dispatchEvent")
                                {
                                    let _: rquickjs::Result<rquickjs::Value> =
                                        dispatch.call((kb_event,));
                                }
                            }
                        }
                    }
                })
            });
        }
    }

    pub fn dispatch_pointer_event(
        &self,
        index: usize,
        type_: &str,
        x: f32,
        y: f32,
        button: i32,
        pointer_id: i32,
        pointer_type: &str,
        is_primary: bool,
    ) {
        if let Some(ref dom_arc) = *self.dom.lock().unwrap() {
            let dom = dom_arc.clone();
            self.with_context(|ctx| {
                ctx.with(|ctx| {
                    use crate::ace::runtime::bindings::html::element::Element;
                    use crate::ace::runtime::bindings::html::event_subclasses::PointerEvent;

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

                        // Create PointerEvent options object
                        let opts = rquickjs::Object::new(ctx.clone()).unwrap();
                        let _ = opts.set("clientX", x as f64);
                        let _ = opts.set("clientY", y as f64);
                        let _ = opts.set("button", button);
                        let _ = opts.set("pointerId", pointer_id);
                        let _ = opts.set("pointerType", pointer_type);
                        let _ = opts.set("isPrimary", is_primary);
                        let _ = opts.set("bubbles", true);

                        if let Ok(pointer_event) = rquickjs::Class::instance(
                            ctx.clone(),
                            PointerEvent::new(type_.to_string(), Some(opts.into_value())),
                        ) {
                            if let Some(obj) = instance_val.as_object() {
                                if let Ok(dispatch) =
                                    obj.get::<_, rquickjs::Function>("dispatchEvent")
                                {
                                    let _: rquickjs::Result<rquickjs::Value> =
                                        dispatch.call((pointer_event,));
                                }
                            }
                        }
                    }
                })
            });
        }
    }

    pub fn dispatch_touch_event(&self, index: usize, type_: &str, x: f32, y: f32, identifier: i32) {
        if let Some(ref dom_arc) = *self.dom.lock().unwrap() {
            let dom = dom_arc.clone();
            self.with_context(|ctx| {
                ctx.with(|ctx| {
                    use crate::ace::runtime::bindings::html::element::Element;
                    use crate::ace::runtime::bindings::html::event_subclasses::TouchEvent;

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

                        // Create TouchEvent options object
                        let opts = rquickjs::Object::new(ctx.clone()).unwrap();
                        let _ = opts.set("clientX", x as f64);
                        let _ = opts.set("clientY", y as f64);
                        let _ = opts.set("identifier", identifier);
                        let _ = opts.set("bubbles", true);
                        let _ = opts.set("cancelable", true);

                        if let Ok(touch_event) = rquickjs::Class::instance(
                            ctx.clone(),
                            TouchEvent::new(type_.to_string(), Some(opts.into_value())),
                        ) {
                            if let Some(obj) = instance_val.as_object() {
                                if let Ok(dispatch) =
                                    obj.get::<_, rquickjs::Function>("dispatchEvent")
                                {
                                    let _: rquickjs::Result<rquickjs::Value> =
                                        dispatch.call((touch_event,));
                                }
                            }
                        }
                    }
                })
            });
        }
    }

    pub fn dispatch_message_event(
        &self,
        message_json: String,
        origin: String,
        _source_rt_id: Option<usize>,
    ) {
        println!(
            "[JsRuntime::dispatch_message_event] Entering (ID {})",
            self.id
        );
        if self.dom.lock().unwrap().is_some() {
            println!("[JsRuntime::dispatch_message_event] DOM exists, getting context lock...");
            self.with_context(|ctx| {
                println!(
                    "[JsRuntime::dispatch_message_event] Context locked, evaluating script..."
                );
                ctx.with(|ctx| {
                    let safe_msg = message_json.replace("'", "\\'");
                    let safe_origin = origin.replace("'", "\\'");
                    let options_str =
                        format!("{{ data: '{}', origin: '{}' }}", safe_msg, safe_origin);
                    let script = format!(
                        "globalThis.dispatchEvent(new MessageEvent('message', {}))",
                        options_str
                    );

                    let _ = ctx.eval::<(), _>(script);
                })
            });
            println!("[JsRuntime::dispatch_message_event] Done.");
        } else {
            println!("[JsRuntime::dispatch_message_event] NO DOM!");
        }
    }

    pub fn get_pending_navigation(&self) -> Option<String> {
        let mut pending = self.pending_navigation.lock().unwrap();
        pending.take()
    }

    pub fn check_same_origin(&self, other: &JsRuntime) -> bool {
        let o1_lock = self.origin.lock().unwrap();
        let o2_lock = other.origin.lock().unwrap();
        match (&*o1_lock, &*o2_lock) {
            (Some(o1), Some(o2)) => o1.is_same_origin(o2),
            _ => false,
        }
    }

    pub fn run_gc(&self) {
        let rt = self.runtime.lock().unwrap();
        rt.run_gc();
    }

    pub fn run_pending(&self) -> (bool, bool) {
        super::executor::run_pending(self)
    }

    pub fn run_raf_callbacks(&self, timestamp: f64) -> bool {
        let callbacks = {
            let mut event_loop = self.event_loop.lock().unwrap();
            event_loop.take_raf_callbacks()
        };

        if callbacks.is_empty() {
            return false;
        }

        self.with_context(|ctx| {
            ctx.with(|ctx| {
                for callback in callbacks {
                    if let Ok(func) = callback.0.restore(&ctx) {
                        let _: rquickjs::Result<rquickjs::Value> = func.call((timestamp,));
                    }
                }
            });
        });

        true
    }

    pub fn run_idle_callbacks(&self, frame_deadline: std::time::Instant) -> bool {
        let callbacks = {
            let mut el = self.event_loop.lock().unwrap();
            el.take_idle_callbacks(frame_deadline)
        };

        if callbacks.is_empty() {
            return false;
        }

        self.with_context(|ctx| {
            ctx.with(|ctx| {
                for task in callbacks {
                    let now = std::time::Instant::now();

                    // Calcular timeRemaining em double (ms)
                    let time_remaining_ms = if frame_deadline > now {
                        frame_deadline.duration_since(now).as_secs_f64() * 1000.0
                    } else {
                        0.0
                    };

                    // Verificar se foi timeout
                    let did_timeout = task.timeout_deadline.map(|d| now >= d).unwrap_or(false);

                    // Criar um IdleDeadline faked com JS wrapper
                    let script = format!(
                        "(function(cb) {{ 
                            var deadline = {{ 
                                timeRemaining: function() {{ return {:.3}; }}, 
                                didTimeout: {} 
                            }};
                            cb(deadline);
                        }})",
                        time_remaining_ms.max(0.0),
                        did_timeout
                    );

                    if let Ok(wrapper_fn) = ctx.eval::<rquickjs::Function, _>(script) {
                        if let Ok(cb) = task.callback.0.restore(&ctx) {
                            let _: rquickjs::Result<Value> = wrapper_fn.call((cb,));
                        }
                    }
                }
            })
        });

        true
    }
}

unsafe impl Send for JsRuntime {}
unsafe impl Sync for JsRuntime {}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
