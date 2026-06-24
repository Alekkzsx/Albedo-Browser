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
            .expect("Albedo Engine: internal invariant violated"),
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
            let runtime = rt.runtime.lock().unwrap_or_else(|e| e.into_inner());
            runtime.set_interrupt_handler(Some(Box::new(move || {
                interceptor_clone.handle_interrupt_no_ctx()
            })));
        }

        Ok(rt)
    }

    /// TODO: add docs
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
}
