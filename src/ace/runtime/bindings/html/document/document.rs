use super::*;
use super::element::Element;
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use crate::ace::engine::style::Stylesheet;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};


#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Document {
    #[qjs(skip_trace)]
    pub dom: Arc<Mutex<AceDOM>>,
    #[qjs(skip_trace)]
    pub stylesheet: Arc<Mutex<Stylesheet>>,
    #[qjs(skip_trace)]
    pub mutations: Arc<Mutex<bool>>,
    #[qjs(skip_trace)]
    pub stylesheet_dirty: Arc<Mutex<bool>>,
    #[qjs(skip_trace)]
    pub primitives: Arc<Mutex<Vec<crate::ace::engine::ACEPrimitive>>>,
    #[qjs(skip_trace)]
    pub canvas_contexts:
        Arc<Mutex<std::collections::HashMap<usize, crate::ace::engine::graphics::canvas2d::Canvas2D>>>,
    #[qjs(skip_trace)]
    pub pending_scroll: Arc<Mutex<Option<usize>>>,
    #[qjs(skip_trace)]
    pub cookie_storage: Arc<Mutex<String>>,
    #[qjs(skip_trace)]
    pub resource_manager: Option<crate::network::resources::ResourceManager>,
    pub url: String,
    pub referrer: String,
    #[qjs(skip_trace)]
    pub element_geometry:
        Arc<Mutex<std::collections::HashMap<usize, crate::ace::engine::ElementGeometry>>>,
    #[qjs(skip_trace)]
    pub element_scroll: Arc<Mutex<std::collections::HashMap<usize, (f32, f32)>>>,
    #[qjs(skip_trace)]
    pub ready_state_ptr: Arc<Mutex<String>>,
}
