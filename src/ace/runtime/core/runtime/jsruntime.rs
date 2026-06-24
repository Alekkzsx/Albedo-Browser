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

unsafe impl Send for JsRuntime {}
unsafe impl Sync for JsRuntime {}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
