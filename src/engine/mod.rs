pub mod dom;
pub mod style;
pub mod graphics;
pub mod svg;
pub mod text;

use std::sync::{Arc, Mutex};
use self::dom::AceDOM;
use self::style::Stylesheet;
use self::style::css_values::{CssAlignItems, CssAlignContent, CssBoxSizing, CssFontWeight, CssFilter, TransformFunction};

#[derive(Clone, Debug, rquickjs::class::Trace)]
pub struct ACEPrimitive {
    pub node_idx: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[qjs(skip_trace)]
    pub style: crate::engine::style::Declaration,
}

pub struct AceEngine {
    pub dom: Option<Arc<Mutex<AceDOM>>>,
    pub stylesheet: Arc<Mutex<Stylesheet>>,
    pub primitives: Arc<Mutex<Vec<ACEPrimitive>>>,
    pub resource_manager: Option<crate::network::resources::ResourceManager>,
    pub current_url: String,
    pub js_runtime: Option<crate::runtime::core::runtime::JsRuntime>,
    pub hovered_element: Option<usize>,
    pub active_element: Option<usize>,
    pub image_cache: Arc<Mutex<std::collections::HashMap<String, slint::Image>>>,
}

impl AceEngine {
    pub fn new() -> Self {
        Self {
            dom: Some(Arc::new(Mutex::new(AceDOM::new()))),
            stylesheet: Arc::new(Mutex::new(Stylesheet::new())),
            primitives: Arc::new(Mutex::new(Vec::new())),
            resource_manager: None,
            current_url: String::new(),
            js_runtime: None,
            hovered_element: None,
            active_element: None,
            image_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn load_url(&mut self, url: &str) {
        println!("Engine loading URL: {}", url);
    }

    pub fn layout(&mut self, width: f32, height: f32) {
        if let Some(ref dom) = self.dom {
            let dom = dom.lock().unwrap();
            let stylesheet = self.stylesheet.lock().unwrap();
            let mut primitives = self.primitives.lock().unwrap();

            // 1. Resolve Styles
            // 2. Build Taffy Tree
            // 3. Compute Layout
            // 4. Update Primitives

            primitives.clear();
            // Simple stub for now
        }
    }

    pub fn set_resource_manager(&mut self, rm: crate::network::resources::ResourceManager) {
        self.resource_manager = Some(rm);
    }

    pub fn find_element_at_position(&self, _x: f32, _y: f32) -> Option<usize> {
        // Stub: retorna None por enquanto
        None
    }

    pub fn set_hover(&mut self, node_id: Option<usize>) {
        self.hovered_element = node_id;
    }

    pub fn set_active(&mut self, node_id: Option<usize>) {
        self.active_element = node_id;
    }

    pub fn tick(&mut self, _now: f64) -> bool {
        // Stub: retorna false (sem animações ativas)
        false
    }

    pub fn check_mutations(&mut self) -> (bool, bool) {
        // Stub: retorna (mutated, style_dirty)
        (false, false)
    }

    pub fn update_stylesheet(&mut self) {
        // Stub: atualiza stylesheet
    }

    pub fn recompute_layout(&mut self) {
        // Stub: recomputa layout
    }

    pub fn render_visual(&self) -> Vec<VisualPrimitive> {
        // Stub: retorna lista vazia de primitivas visuais
        Vec::new()
    }

    pub fn process_resource_responses(&mut self) -> bool {
        // Stub: processa respostas de recursos
        false
    }

    pub fn load_html(&mut self, html: &str) {
        use html5ever::parse_document;
        use html5ever::tendril::TendrilSink;
        use kuchiki::parse_html;
        
        let dom_tree = parse_html().one(html);
        self.dom = Some(Arc::new(Mutex::new(AceDOM::from_kuchiki(dom_tree))));
    }
}

impl Clone for AceEngine {
    fn clone(&self) -> Self {
        Self {
            dom: self.dom.clone(),
            stylesheet: self.stylesheet.clone(),
            primitives: self.primitives.clone(),
            resource_manager: self.resource_manager.clone(),
            current_url: self.current_url.clone(),
            js_runtime: self.js_runtime.clone(),
            hovered_element: self.hovered_element,
            active_element: self.active_element,
            image_cache: self.image_cache.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct VisualPrimitive {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: String,
    pub text: String,
    pub font_size: f32,
    pub image_url: Option<String>,
    pub link_url: Option<String>,
    pub node_idx: usize,
    pub element_type: String,
    pub is_fixed: bool,
}
