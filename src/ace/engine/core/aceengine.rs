use super::*;
use crate::ace::engine::graphics::compositor;

use crate::ace::engine::dom::AceDOM;
use crate::ace::engine::style::css_values::ComputedStyle;
use crate::ace::engine::style::Stylesheet;
use crate::ace::engine::text::TextMeasurer;
use crate::ace::engine::layout::{ElementGeometry, ACEPrimitive, InvalidationManager};
use crate::utils::time::unix_timestamp_secs_f64;
use std::sync::{Arc, Mutex};


pub struct AceEngine {
    pub dom: Option<Arc<Mutex<AceDOM>>>,
    pub stylesheet: Arc<Mutex<Stylesheet>>,
    pub primitives: Arc<Mutex<Vec<ACEPrimitive>>>,
    pub resource_manager: Option<crate::network::resources::ResourceManager>,
    pub current_url: String,
    pub js_runtime: Option<crate::ace::runtime::core::runtime::JsRuntime>,
    pub hovered_element: Option<usize>,
    pub active_element: Option<usize>,
    pub focused_element: Option<usize>,
    pub image_cache: Arc<Mutex<std::collections::HashMap<String, slint::Image>>>,
    pub element_geometry: Arc<Mutex<std::collections::HashMap<usize, ElementGeometry>>>,
    pub element_scroll: Arc<Mutex<std::collections::HashMap<usize, (f32, f32)>>>,
    pub styles_dirty: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub animation_manager: Arc<Mutex<crate::ace::engine::style::animation::AnimationManager>>,
    pub canvas_contexts:
        Arc<Mutex<std::collections::HashMap<usize, crate::ace::engine::graphics::canvas2d::Canvas2D>>>,
    pub taffy: Arc<Mutex<taffy::Taffy>>,
    pub viewport_y: f32,
    /// Geometrias de elementos em subframes projetadas para o espaço global do frame pai.
    /// Chave: (iframe_node_idx * 1_000_000) + elem_node_idx_no_subframe
    /// Atualizado por collect_subframe_geometries() a cada layout.
    pub iframe_projected_geometry: Arc<Mutex<std::collections::HashMap<u64, ElementGeometry>>>,
    pub external_css: Arc<Mutex<std::collections::HashMap<String, String>>>,
    pub pending_resources: Arc<Mutex<std::collections::HashSet<String>>>,
    pub element_styles: Arc<Mutex<std::collections::HashMap<usize, ComputedStyle>>>,
    pub node_to_taffy: Arc<Mutex<std::collections::HashMap<usize, taffy::prelude::Node>>>,
    pub text_measurer: TextMeasurer,
    pub framebuffer: Arc<Mutex<Option<tiny_skia::Pixmap>>>,
    pub invalidation_manager: Arc<Mutex<InvalidationManager>>,
    pub font_system: Arc<Mutex<cosmic_text::FontSystem>>,
    pub swash_cache: Arc<Mutex<cosmic_text::SwashCache>>,
    pub gpu_compositor: std::sync::Arc<tokio::sync::Mutex<Option<compositor::GpuCompositor>>>,
}

impl AceEngine {
    /// TODO: add docs
    pub fn new() -> Self {
        let font_system = Arc::new(Mutex::new(cosmic_text::FontSystem::new()));
        let swash_cache = Arc::new(Mutex::new(cosmic_text::SwashCache::new()));
        let engine = Self {
            dom: Some(Arc::new(Mutex::new(AceDOM::new()))),
            stylesheet: Arc::new(Mutex::new(crate::ace::engine::style::get_user_agent_stylesheet())),
            primitives: Arc::new(Mutex::new(Vec::new())),
            resource_manager: None,
            current_url: String::new(),
            js_runtime: None,
            hovered_element: None,
            active_element: None,
            focused_element: None,
            image_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
            element_geometry: Arc::new(Mutex::new(std::collections::HashMap::new())),
            element_scroll: Arc::new(Mutex::new(std::collections::HashMap::new())),
            styles_dirty: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
            animation_manager: Arc::new(Mutex::new(
                crate::ace::engine::style::animation::AnimationManager::new(),
            )),
            canvas_contexts: Arc::new(Mutex::new(std::collections::HashMap::new())),
            taffy: Arc::new(Mutex::new(taffy::Taffy::new())),
            viewport_y: 0.0,
            iframe_projected_geometry: Arc::new(Mutex::new(std::collections::HashMap::new())),
            external_css: Arc::new(Mutex::new(std::collections::HashMap::new())),
            pending_resources: Arc::new(Mutex::new(std::collections::HashSet::new())),
            element_styles: Arc::new(Mutex::new(std::collections::HashMap::new())),
            node_to_taffy: Arc::new(Mutex::new(std::collections::HashMap::new())),
            text_measurer: TextMeasurer::new(font_system.clone()),
            framebuffer: Arc::new(Mutex::new(None)),
            invalidation_manager: Arc::new(Mutex::new(InvalidationManager::new())),
            font_system: font_system,
            swash_cache,
            gpu_compositor: Arc::new(tokio::sync::Mutex::new(None)),
        };
        engine.init_gpu();
        engine
    }

    /// TODO: add docs
    pub fn init_gpu(&self) {
        let comp_arc = self.gpu_compositor.clone();
        tokio::spawn(async move {
            tracing::info!("Initializing GPU Compositor (WGPU)");
            if let Some(comp) = compositor::GpuCompositor::new().await {
                let mut lock = comp_arc.lock().await;
                *lock = Some(comp);
                tracing::info!("GPU Compositor initialized successfully");
            } else {
                tracing::warn!("Failed to initialize GPU Compositor, falling back to CPU");
            }
        });
    }

    /// TODO: add docs
    pub fn set_resource_manager(&mut self, rm: crate::network::resources::ResourceManager) {
        self.resource_manager = Some(rm);
    }

    /// TODO: add docs
    pub fn with_js_context<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&crate::ace::runtime::core::runtime::JsRuntime, &AceDOM) -> R,
    {
        let rt = self.js_runtime.as_ref()?;
        let dom_arc = self.dom.as_ref()?;
        let dom = dom_arc.lock().unwrap_or_else(|e| e.into_inner());
        Some(f(rt, &dom))
    }

    /// TODO: add docs
    pub fn set_hover(&mut self, node_id: Option<usize>) {
        self.hovered_element = node_id;
        self.mark_styles_dirty();
    }

    /// TODO: add docs
    pub fn set_active(&mut self, node_id: Option<usize>) {
        self.active_element = node_id;
        self.mark_styles_dirty();
    }

    /// TODO: add docs
    pub fn set_focused(&mut self, node_id: Option<usize>) {
        self.focused_element = node_id;
        self.mark_styles_dirty();
    }

    /// TODO: add docs
    pub fn mark_styles_dirty(&mut self) {
        tracing::debug!("mark_styles_dirty called");
        self.styles_dirty
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// TODO: add docs
    include!("aceengine_styles.rs");

    /// TODO: add docs
    pub fn update_element_bounds(&self, node_idx: usize, x: f32, y: f32, width: f32, height: f32) {
        let mut geometry = self.element_geometry.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(geom) = geometry.get_mut(&node_idx) {
            geom.x = x;
            geom.y = y;
            geom.width = width;
            geom.height = height;
        } else {
            let mut geom = ElementGeometry::new();
            geom.x = x;
            geom.y = y;
            geom.width = width;
            geom.height = height;
            geometry.insert(node_idx, geom);
        }
    }

    /// TODO: add docs
    pub fn clear_element_bounds(&self) {
        let mut geometry = self.element_geometry.lock().unwrap_or_else(|e| e.into_inner());
        geometry.clear();
    }

}

impl Clone for AceEngine {
pub(crate) fn clone(&self) -> Self {
        Self {
            dom: self.dom.clone(),
            stylesheet: self.stylesheet.clone(),
            primitives: self.primitives.clone(),
            resource_manager: self.resource_manager.clone(),
            iframe_projected_geometry: self.iframe_projected_geometry.clone(),
            current_url: self.current_url.clone(),
            js_runtime: self.js_runtime.clone(),
            hovered_element: self.hovered_element,
            active_element: self.active_element,
            focused_element: self.focused_element,
            image_cache: self.image_cache.clone(),
            element_geometry: self.element_geometry.clone(),
            element_scroll: self.element_scroll.clone(),
            styles_dirty: self.styles_dirty.clone(),
            animation_manager: self.animation_manager.clone(),
            canvas_contexts: self.canvas_contexts.clone(),
            taffy: self.taffy.clone(),
            viewport_y: self.viewport_y,
            node_to_taffy: self.node_to_taffy.clone(),
            external_css: self.external_css.clone(),
            pending_resources: self.pending_resources.clone(),
            element_styles: self.element_styles.clone(),
            text_measurer: self.text_measurer.clone(),
            framebuffer: self.framebuffer.clone(),
            invalidation_manager: self.invalidation_manager.clone(),
            font_system: self.font_system.clone(),
            swash_cache: self.swash_cache.clone(),
            gpu_compositor: self.gpu_compositor.clone(),
        }
    }
}

impl std::fmt::Debug for AceEngine {
pub(crate) fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AceEngine")
            .field("current_url", &self.current_url)
            .field("styles_dirty", &self.styles_dirty)
            .finish()
    }
}
