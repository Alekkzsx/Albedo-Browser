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

    pub fn init_gpu(&self) {
        let comp_arc = self.gpu_compositor.clone();
        tokio::spawn(async move {
            println!("[AceEngine] Initializing GPU Compositor (WGPU)...");
            if let Some(comp) = compositor::GpuCompositor::new().await {
                let mut lock = comp_arc.lock().await;
                *lock = Some(comp);
                println!("[AceEngine] GPU Compositor initialized successfully!");
            } else {
                println!("[AceEngine] WARNING: Failed to initialize GPU Compositor. Falling back to CPU bounds.");
            }
        });
    }

    pub fn set_resource_manager(&mut self, rm: crate::network::resources::ResourceManager) {
        self.resource_manager = Some(rm);
    }

    pub fn set_hover(&mut self, node_id: Option<usize>) {
        self.hovered_element = node_id;
        self.mark_styles_dirty();
    }

    pub fn set_active(&mut self, node_id: Option<usize>) {
        self.active_element = node_id;
        self.mark_styles_dirty();
    }

    pub fn set_focused(&mut self, node_id: Option<usize>) {
        self.focused_element = node_id;
        self.mark_styles_dirty();
    }

    pub fn mark_styles_dirty(&mut self) {
        println!("[DEBUG] mark_styles_dirty called! Stack trace or origin unknown.");
        self.styles_dirty
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn recompute_dirty_styles(&mut self) {
        if !self.styles_dirty.load(std::sync::atomic::Ordering::SeqCst) {
            // Check if any individual node is STYLE dirty
            let is_any_node_style_dirty = if let Some(ref dom_arc) = self.dom {
                let dom = dom_arc.lock().unwrap();
                dom.nodes
                    .iter()
                    .any(|n| n.dirty.contains(crate::ace::engine::dom::NodeDirtyFlags::STYLE))
            } else {
                false
            };
            if !is_any_node_style_dirty {
                return;
            }
        }

        println!("[AceEngine] Actual Recompute of dirty styles starting...");

        // Recompilar estilos para toda a árvore DOM
        if let Some(ref dom_arc) = self.dom {
            let dom = dom_arc.lock().unwrap();
            let stylesheet = self.stylesheet.lock().unwrap();

            let now = unix_timestamp_secs_f64();
            let mut am = self.animation_manager.lock().unwrap();

            // Map index -> ComputedStyle for inheritance
            let mut style_cache: std::collections::HashMap<usize, ComputedStyle> =
                std::collections::HashMap::new();

            // Recompilar estilos com novos estados de hover/focus/active
            for idx in 0..dom.nodes.len() {
                let parent_style = if let Some(parent_idx) = dom.nodes[idx].parent {
                    style_cache.get(&parent_idx)
                } else {
                    None
                };

                let computed = stylesheet.calculate_style(
                    &dom,
                    idx,
                    parent_style,
                    None,
                    self.hovered_element,
                    self.focused_element,
                    self.active_element,
                    Some(&am),
                    now,
                    800.0,
                    600.0,
                    "light",
                );

                // Start Keyframe Animations
                if let crate::ace::engine::dom::AceNodeType::Element(_) = &dom.nodes[idx].node_type {
                    for anim_def in &computed.animations {
                        if let Some(keyframes) = stylesheet.keyframes.get(&anim_def.name) {
                            let timing = crate::ace::engine::style::animation::TimingFunction::Ease;
                            am.start_keyframe_animation(
                                idx,
                                anim_def.name.clone(),
                                keyframes.clone(),
                                anim_def.duration_ms as f64 / 1000.0,
                                timing,
                                now,
                            );
                        }
                    }
                }

                style_cache.insert(idx, computed);
            }

            // Persistir estilos computados no cache da engine
            {
                let mut engine_styles = self.element_styles.lock().unwrap();
                *engine_styles = style_cache;
            }
        }

        self.styles_dirty
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn update_element_bounds(&self, node_idx: usize, x: f32, y: f32, width: f32, height: f32) {
        let mut geometry = self.element_geometry.lock().unwrap();
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

    pub fn clear_element_bounds(&self) {
        let mut geometry = self.element_geometry.lock().unwrap();
        geometry.clear();
    }

}

impl Clone for AceEngine {
    fn clone(&self) -> Self {
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AceEngine")
            .field("current_url", &self.current_url)
            .field("styles_dirty", &self.styles_dirty)
            .finish()
    }
}
