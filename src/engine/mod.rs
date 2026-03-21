pub mod compositor;
pub mod dom;
pub mod graphics;
pub mod inline;
pub mod layer_tree;
pub mod style;
pub mod svg;
pub mod text;
pub mod types;
use self::style::css_values::{ComputedStyle, CssFilter, TransformFunction};
use self::style::Stylesheet;
use self::text::TextMeasurer;
use crate::engine::dom::AceDOM;
use std::sync::{Arc, Mutex};
use taffy::geometry::MinMax;
use taffy::prelude::*;

/// Complete element geometry information including scroll and content dimensions
#[derive(Clone, Debug)]
pub struct ElementGeometry {
    // Layout position and size (from Taffy)
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,

    // Border and padding (from computed style)
    pub border_top: f32,
    pub border_right: f32,
    pub border_bottom: f32,
    pub border_left: f32,
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,

    // Scroll offset for this element
    pub scroll_x: f32,
    pub scroll_y: f32,

    // Content dimensions (min/max bounds of children)
    pub content_width: f32,
    pub content_height: f32,

    // Overflow style
    pub overflow_x: crate::engine::style::css_values::CssOverflow,
    pub overflow_y: crate::engine::style::css_values::CssOverflow,

    // Float & Clear context
    pub float: crate::engine::style::css_values::CssFloat,
    pub clear: crate::engine::style::css_values::CssClear,
}

impl ElementGeometry {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            border_top: 0.0,
            border_right: 0.0,
            border_bottom: 0.0,
            border_left: 0.0,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            content_width: 0.0,
            content_height: 0.0,
            overflow_x: crate::engine::style::css_values::CssOverflow::Visible,
            overflow_y: crate::engine::style::css_values::CssOverflow::Visible,
            float: crate::engine::style::css_values::CssFloat::None,
            clear: crate::engine::style::css_values::CssClear::None,
        }
    }

    /// Client width: content width - padding (no border) - Actually width - borders
    pub fn client_width(&self) -> f32 {
        (self.width - self.border_left - self.border_right).max(0.0)
    }

    /// Client height: content height - padding (no border) - Actually height - borders
    pub fn client_height(&self) -> f32 {
        (self.height - self.border_top - self.border_bottom).max(0.0)
    }

    /// Scroll width: max of client width and content width
    pub fn scroll_width(&self) -> f32 {
        self.content_width.max(self.client_width())
    }

    /// Scroll height: max of client height and content height
    pub fn scroll_height(&self) -> f32 {
        self.content_height.max(self.client_height())
    }
}

#[derive(Clone)]
pub struct GridContext {
    pub column_names: std::collections::HashMap<String, Vec<i16>>,
    pub row_names: std::collections::HashMap<String, Vec<i16>>,
    pub areas: std::collections::HashMap<String, (usize, usize, usize, usize)>,
    pub col_offset: i16,
    pub row_offset: i16,
}

#[derive(Clone, Debug, rquickjs::class::Trace)]
pub struct ACEPrimitive {
    pub node_idx: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[qjs(skip_trace)]
    pub style: crate::engine::style::Declaration,
    // Form Extensions
    pub input_value: String,
    pub placeholder: String,
    pub input_type: String,
    pub options: String,
}

/// Complete element geometry information including scroll and content dimensions
#[derive(Clone, Debug, Default)]
pub struct InvalidationManager {
    pub dirty_rects: Vec<tiny_skia::Rect>,
}

impl InvalidationManager {
    pub fn new() -> Self {
        Self {
            dirty_rects: Vec::new(),
        }
    }

    pub fn add_dirty_rect(&mut self, rect: tiny_skia::Rect) {
        let mut merged = false;
        for existing in &mut self.dirty_rects {
            if let Some(union_rect) = Self::union_rect(*existing, rect) {
                *existing = union_rect;
                merged = true;
                break;
            }
        }
        if !merged {
            self.dirty_rects.push(rect);
        }
    }

    pub fn union_rect(a: tiny_skia::Rect, b: tiny_skia::Rect) -> Option<tiny_skia::Rect> {
        let left = a.left().min(b.left());
        let right = a.right().max(b.right());
        let top = a.top().min(b.top());
        let bottom = a.bottom().max(b.bottom());

        tiny_skia::Rect::from_ltrb(left, top, right, bottom)
    }

    pub fn clear(&mut self) {
        self.dirty_rects.clear();
    }
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
    pub focused_element: Option<usize>,
    pub image_cache: Arc<Mutex<std::collections::HashMap<String, slint::Image>>>,
    pub element_geometry: Arc<Mutex<std::collections::HashMap<usize, ElementGeometry>>>,
    pub element_scroll: Arc<Mutex<std::collections::HashMap<usize, (f32, f32)>>>,
    pub styles_dirty: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub animation_manager: Arc<Mutex<crate::engine::style::animation::AnimationManager>>,
    pub canvas_contexts:
        Arc<Mutex<std::collections::HashMap<usize, crate::engine::graphics::canvas2d::Canvas2D>>>,
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
            stylesheet: Arc::new(Mutex::new(crate::engine::style::get_user_agent_stylesheet())),
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
                crate::engine::style::animation::AnimationManager::new(),
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

    pub fn load_url(&mut self, url: String) {
        println!("[AceEngine] Initiating load for URL: {}", url);
        self.current_url = url.clone();

        if let Some(rm) = &self.resource_manager {
            rm.fetch(url, crate::network::resources::ResourceType::Html, None);
        }
    }

    pub fn handle_resource_response(
        &mut self,
        res: crate::network::resources::ResourceResponse,
    ) -> bool {
        let mut needs_layout = false;

        if res.url == self.current_url {
            if let Ok(html) = String::from_utf8(res.data.clone()) {
                println!("[AceEngine] Main document downloaded. Calling process_html...");
                self.load_html(&html);
                println!("[AceEngine] process_html returned successfully.");
                needs_layout = true;
            }
        } else {
            let (is_css, css_data) = {
                let mut pending = self.pending_resources.lock().unwrap();
                if pending.contains(&res.url) {
                    pending.remove(&res.url);
                    (true, Some(res.data.clone()))
                } else {
                    (false, None)
                }
            };

            if is_css {
                if let Some(data) = css_data {
                    if let Ok(css) = String::from_utf8(data) {
                        println!("[AceEngine] External CSS downloaded: {}", res.url);
                        {
                            let mut external = self.external_css.lock().unwrap();
                            external.insert(res.url.clone(), css);
                        }
                        // Trigger style recomputation after dropping lock
                        self.update_stylesheet();
                        needs_layout = true;
                    }
                }
            }
        }

        // Push response down to subframes to check if it's theirs
        if let Some(ref dom_arc) = self.dom {
            let dom = dom_arc.lock().unwrap();
            if let Some(ref subframes_arc) = dom.subframes {
                let mut subframes = subframes_arc.lock().unwrap();
                for (_, sub_engine_arc) in subframes.iter_mut() {
                    let mut sub_engine = sub_engine_arc.lock().unwrap();
                    if sub_engine.handle_resource_response(res.clone()) {
                        needs_layout = true;
                    }
                }
            }
        }

        needs_layout
    }

    pub fn layout(&mut self, width: f32, height: f32) {
        if let Some(ref dom_arc) = self.dom {
            let mut dom = dom_arc.lock().unwrap();
            let stylesheet = self.stylesheet.lock().unwrap();

            let mut taffy = self.taffy.lock().unwrap();
            taffy.clear();

            let mut node_map = std::collections::HashMap::new();

            // 1. Build Taffy Tree starting from root (index 0)
            // Assuming index 0 is always Document or root element
            if !dom.nodes.is_empty() {
                let root_nodes = self.build_layout_tree(
                    &mut dom,
                    &mut taffy,
                    &stylesheet,
                    0,
                    width,
                    height,
                    &mut node_map,
                    None,
                );

                if let Some(&root_node) = root_nodes.first() {
                    let available_space = taffy::prelude::Size {
                        width: taffy::prelude::AvailableSpace::Definite(width),
                        height: taffy::prelude::AvailableSpace::Definite(height),
                    };

                    // 2. Compute Layout
                    if let Ok(_) = taffy.compute_layout(root_node, available_space) {
                        let mut geometry = self.element_geometry.lock().unwrap();
                        geometry.clear();

                        // 3. Extract Global Coordinates
                        self.extract_layout_recursively(
                            &taffy,
                            root_node,
                            &node_map,
                            &mut geometry,
                            0.0,
                            0.0,
                        );
                    } else {
                        println!("[AceEngine] Layout computation failed");
                    }
                }
            }
        }
    }

    fn extract_layout_recursively(
        &self,
        taffy: &taffy::Taffy,
        node: taffy::prelude::Node,
        node_map: &std::collections::HashMap<taffy::prelude::Node, usize>,
        geometry: &mut std::collections::HashMap<usize, ElementGeometry>,
        parent_x: f32,
        parent_y: f32,
    ) {
        if let Ok(layout) = taffy.layout(node) {
            let x = parent_x + layout.location.x;
            let y = parent_y + layout.location.y;
            let w = layout.size.width;
            let h = layout.size.height;

            if let Some(&dom_idx) = node_map.get(&node) {
                let mut geom = ElementGeometry::new();
                geom.x = x;
                geom.y = y;
                geom.width = w;
                geom.height = h;
                // TODO: Extract borders, padding, content dimensions, overflow styles from DOM
                geometry.insert(dom_idx, geom);
            }

            if let Ok(children) = taffy.children(node) {
                for child in children {
                    self.extract_layout_recursively(taffy, child, node_map, geometry, x, y);
                }
            }
        }
    }

    /// Extract box model (borders and padding) from element.
    /// Returns: (border_top, border_right, border_bottom, border_left, padding_top, padding_right, padding_bottom, padding_left)
    fn extract_box_model(&self, _node_idx: usize) -> (f32, f32, f32, f32, f32, f32, f32, f32) {
        // TODO: Read from DOM computed style
        // For now, return zeros
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    }

    /// Calculate content dimensions based on children bounds.
    /// Returns: (content_width, content_height)
    fn calculate_content_dimensions(&self, _node_idx: usize) -> (f32, f32) {
        // TODO: Iterate children and find max bounds
        // For now, return zeros
        (0.0, 0.0)
    }

    /// Get overflow style (overflow-x, overflow-y) from element.
    /// Returns: (String, String)
    fn get_overflow_style(&self, _node_idx: usize) -> (String, String) {
        // TODO: Read from DOM computed style
        // For now, return "visible" for both
        ("visible".to_string(), "visible".to_string())
    }

    pub fn set_resource_manager(&mut self, rm: crate::network::resources::ResourceManager) {
        self.resource_manager = Some(rm);
    }

    pub fn find_element_at_position(&self, x: f32, y: f32) -> Option<usize> {
        // Hit testing: encontra o elemento no topo na posição (x, y)
        // Busca em ordem reversa (z-index maior = renderizado por último = no topo)
        let geometry = self.element_geometry.lock().unwrap();
        let mut topmost: Option<(usize, f32)> = None; // (node_idx, z_index)

        for (node_idx, geom) in geometry.iter() {
            // Verificar se ponto (x, y) está dentro da caixa
            if x >= geom.x && x < (geom.x + geom.width) && y >= geom.y && y < (geom.y + geom.height)
            {
                // Caixas com z-index maior são renderizadas por último
                // Para simplificar, usamos o index como z-order (elementos adicionados depois têm z-index maior)
                let z_index = *node_idx as f32;
                match topmost {
                    None => topmost = Some((*node_idx, z_index)),
                    Some((_, current_z)) if z_index > current_z => {
                        topmost = Some((*node_idx, z_index))
                    }
                    _ => {}
                }
            }
        }

        topmost.map(|(idx, _)| idx)
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
                    .any(|n| n.dirty.contains(crate::engine::dom::NodeDirtyFlags::STYLE))
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

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
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
                if let crate::engine::dom::AceNodeType::Element(_) = &dom.nodes[idx].node_type {
                    for anim_def in &computed.animations {
                        if let Some(keyframes) = stylesheet.keyframes.get(&anim_def.name) {
                            let timing = crate::engine::style::animation::TimingFunction::Ease;
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

    pub fn tick(&mut self, now: f64) -> bool {
        let mut am = self.animation_manager.lock().unwrap();
        let mut has_changes = am.tick(now);

        // Disparar requestAnimationFrame callbacks
        if let Some(ref rt) = self.js_runtime {
            if rt.run_raf_callbacks(now) {
                has_changes = true;
            }
        }

        if has_changes {
            println!("[DEBUG] tick() has_changes == true, setting styles_dirty!");
            self.styles_dirty
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }

        has_changes
    }

    pub fn scroll_into_view(&mut self, node_idx: usize) {
        let geometry = self.element_geometry.lock().unwrap();
        if let Some(geom) = geometry.get(&node_idx) {
            // No Slint, viewport-y é 0 no topo e fica mais negativo à medida que descemos.
            // Para colocar o elemento no topo da visão: viewport_y = -y
            // Para centralizar: viewport_y = -y + (window_height / 2)
            self.viewport_y = -geom.y;
            println!("[AceEngine] Scrolling to node {}: Y={}", node_idx, geom.y);
        }
    }

    pub fn check_mutations(&mut self) -> (bool, bool) {
        let mut mutated = false;
        let style_dirty = false;

        // 1. Verificar pedidos de scroll vindos do JS
        let pending_scroll_idx = if let Some(ref rt) = self.js_runtime {
            let mut ps = rt.pending_scroll.lock().unwrap();
            ps.take()
        } else {
            None
        };

        if let Some(node_idx) = pending_scroll_idx {
            self.scroll_into_view(node_idx);
            mutated = true; // Forçar sincronização com a UI
        }

        // 2. Verificar mutações no DOM
        if let Some(ref dom_arc) = self.dom {
            let _dom = dom_arc.lock().unwrap();
            // A implementação real do AceDOM pode ter flags para isso
            // mutated = dom.has_pending_mutations();
        }

        (mutated, style_dirty)
    }

    pub fn update_stylesheet(&mut self) {
        let mut css_source = String::new();
        if let Some(ref dom_arc) = self.dom {
            let dom = dom_arc.lock().unwrap();
            for node in &dom.nodes {
                if let crate::engine::dom::AceNodeType::Element(el) = &node.node_type {
                    if el.tag == "style" {
                        for &child_idx in &node.children {
                            if let Some(child_node) = dom.get_node(child_idx) {
                                if let crate::engine::dom::AceNodeType::Text(text) =
                                    &child_node.node_type
                                {
                                    css_source.push_str(text.as_ref());
                                    css_source.push_str("\n");
                                }
                            }
                        }
                    } else if el.tag == "link"
                        && el.attributes.get("rel").map(|s| s.as_str()) == Some("stylesheet")
                    {
                        if let Some(href) = el.attributes.get("href") {
                            // Resolve relative URL
                            if let Ok(base_url) = url::Url::parse(&self.current_url) {
                                if let Ok(abs_url) = base_url.join(href) {
                                    let url_str = abs_url.to_string();

                                    // Check if we already have it
                                    let external = self.external_css.lock().unwrap();
                                    if let Some(content) = external.get(&url_str) {
                                        css_source.push_str(content);
                                        css_source.push_str("\n");
                                    } else {
                                        // Trigger download if not pending
                                        let mut pending = self.pending_resources.lock().unwrap();
                                        if !pending.contains(&url_str) {
                                            println!("[AceEngine] Triggering download for external CSS: {}", url_str);
                                            if let Some(ref rm) = self.resource_manager {
                                                rm.fetch(
                                                    url_str.clone(),
                                                    crate::network::resources::ResourceType::Css,
                                                    None,
                                                );
                                            }
                                            pending.insert(url_str);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let new_stylesheet = crate::engine::style::parse(&css_source);
        println!(
            "[AceEngine] Parsed Author CSS, {} bytes injected. Rules fetched: {}, UA: {}",
            css_source.len(),
            new_stylesheet.rules.len(),
            new_stylesheet.user_agent_rules.len()
        );
        let mut current_style = self.stylesheet.lock().unwrap();
        // Copiar TODOS os campos da nova stylesheet (rules, rule_maps, media, supports, container, fonts, keyframes)
        current_style.rules = new_stylesheet.rules;
        current_style.author_rule_map = new_stylesheet.author_rule_map;
        current_style.media_rules = new_stylesheet.media_rules;
        current_style.supports_rules = new_stylesheet.supports_rules;
        current_style.container_rules = new_stylesheet.container_rules;
        current_style.font_faces = new_stylesheet.font_faces;
        current_style.keyframes = new_stylesheet.keyframes;
        println!("[DEBUG] update_stylesheet() setting styles_dirty!");
        self.styles_dirty
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn recompute_layout(&mut self) {
        let start_time = std::time::Instant::now();
        if let Some(ref dom_arc) = self.dom {
            let mut dom = dom_arc.lock().unwrap();
            let mut taffy = self.taffy.lock().unwrap();
            let stylesheet = self.stylesheet.lock().unwrap();

            // 1. Build/Update Taffy Tree Incrementally
            let mut node_map = std::collections::HashMap::new();

            let build_start = std::time::Instant::now();
            let root_nodes = self.build_layout_tree(
                &mut dom,
                &mut taffy,
                &stylesheet,
                0,
                800.0,
                600.0,
                &mut node_map,
                None,
            );
            let build_duration = build_start.elapsed();

            if root_nodes.is_empty() {
                return;
            }
            let root_node = root_nodes[0];

            // 2. Compute Layout
            let compute_start = std::time::Instant::now();
            // We only compute if the root or its descendants are LAYOUT dirty OR if it's the first run
            let size = taffy::geometry::Size {
                width: taffy::style::AvailableSpace::Definite(800.0),
                height: taffy::style::AvailableSpace::MaxContent,
            };
            let _ = taffy.compute_layout(root_node, size);
            let compute_duration = compute_start.elapsed();

            // 3. Update Element Bounds
            self.sync_taffy_bounds(&taffy, &dom, root_node, 0.0, 0.0, &node_map);

            // 4. Reset Dirty Flags
            for node in dom.nodes.iter_mut() {
                node.dirty = crate::engine::dom::NodeDirtyFlags::NONE;
            }

            let total_duration = start_time.elapsed();
            println!(
                "[AceEngine] Layout Recomputed (Incremental): Total={:?}, Build={:?}, Compute={:?}",
                total_duration, build_duration, compute_duration
            );
        }

        // 3.5. Float and Clear CSS apply pass
        self.apply_floats_and_clear();

        // 4. Projetar geometrias dos subframes para o espaço global
        self.collect_subframe_geometries();

        // 5. Sincronizar iframe_projected_geometry com o JsRuntime do frame pai
        if let Some(ref rt) = self.js_runtime {
            let projected = self.iframe_projected_geometry.lock().unwrap();
            let mut rt_projected = rt.iframe_projected_geometry.lock().unwrap();
            *rt_projected = projected.clone();
        }
    }

    /// Percorre todos os subframes e projeta as geometrias dos seus elementos
    /// para o espaço de coordenadas global do frame pai.
    ///
    /// Para um elemento com coordenadas locais (ex, ey) dentro de um iframe
    /// cujo rect no frame pai é (ix, iy, iw, ih), a posição global é:
    ///   global_x = ix + ex
    ///   global_y = iy + ey
    ///
    /// Elementos que estiverem completamente fora do iframe são descartados.
    ///
    /// Chave no map: (iframe_node_idx as u64) * 1_000_000 + (elem_node_idx as u64)
    pub fn collect_subframe_geometries(&self) {
        let mut projected = self.iframe_projected_geometry.lock().unwrap();
        projected.clear();

        let dom_arc = match &self.dom {
            Some(d) => d,
            None => return,
        };

        let dom = dom_arc.lock().unwrap();
        let subframes_arc = match &dom.subframes {
            Some(s) => s.clone(),
            None => return,
        };
        // Drop DOM lock before iterating subframes (evitar deadlock)
        drop(dom);

        let subframes = subframes_arc.lock().unwrap();
        let parent_geometry = self.element_geometry.lock().unwrap();

        for (&iframe_node_idx, sub_engine_arc) in subframes.iter() {
            // Obter rect do <iframe> no frame pai
            let iframe_rect = match parent_geometry.get(&iframe_node_idx) {
                Some(g) => (g.x, g.y, g.width, g.height),
                None => {
                    // iframe ainda não tem geometria calculada — pular
                    continue;
                }
            };
            let (iframe_x, iframe_y, iframe_w, iframe_h) = iframe_rect;

            // Obter geometrias do subframe
            let sub_engine = sub_engine_arc.lock().unwrap();
            let sub_geometry = sub_engine.element_geometry.lock().unwrap();

            for (&elem_idx, elem_geom) in sub_geometry.iter() {
                // Projetar para coordenadas globais
                let global_x = iframe_x + elem_geom.x;
                let global_y = iframe_y + elem_geom.y;

                // Clipping: recortar pelo rect do iframe
                // Se o elemento está completamente fora do iframe, descartamos
                let clip_x1 = global_x.max(iframe_x);
                let clip_y1 = global_y.max(iframe_y);
                let clip_x2 = (global_x + elem_geom.width).min(iframe_x + iframe_w);
                let clip_y2 = (global_y + elem_geom.height).min(iframe_y + iframe_h);

                let _visible_w = (clip_x2 - clip_x1).max(0.0);
                let _visible_h = (clip_y2 - clip_y1).max(0.0);

                // Ainda publicamos mesmo que parcialmente visível
                // (o observer irá calcular a razão de interseção corretamente)
                let mut proj_geom = elem_geom.clone();
                proj_geom.x = global_x;
                proj_geom.y = global_y;
                // Expor as dimensões reais (não clippadas) — o observer clipa

                // Chave única que codifica (iframe, elem)
                let key: u64 = (iframe_node_idx as u64) * 1_000_000 + (elem_idx as u64);
                projected.insert(key, proj_geom);

                // Log apenas para debug em modo verbose
                // println!("[Subframe] iframe={} elem={} -> global=({:.0},{:.0}) visible=({:.0}x{:.0})",
                //     iframe_node_idx, elem_idx, global_x, global_y, visible_w, visible_h);
            }
        }
    }

    pub fn apply_floats_and_clear(&self) {
        let dom_arc = match &self.dom {
            Some(d) => d.clone(),
            None => return,
        };
        let dom = dom_arc.lock().unwrap();

        let mut float_ctx = FloatContext::default();
        let mut geometry = self.element_geometry.lock().unwrap();
        self.apply_floats_recursive(0, &dom, &mut float_ctx, &mut geometry, 0.0);
    }

    fn apply_floats_recursive(
        &self,
        node_idx: usize,
        dom: &crate::engine::AceDOM,
        float_ctx: &mut FloatContext,
        geometry: &mut std::collections::HashMap<usize, ElementGeometry>,
        offset_y: f32,
    ) -> f32 {
        let node = match dom.get_node(node_idx) {
            Some(n) => n,
            None => return offset_y,
        };

        let mut current_offset_y = offset_y;

        if let Some(geom) = geometry.get_mut(&node_idx) {
            let my_float = geom.float.clone();
            let my_clear = geom.clear.clone();

            geom.y += current_offset_y;

            if my_clear != crate::engine::style::css_values::CssClear::None {
                let new_y = float_ctx.get_clear_y(&my_clear, geom.y);
                if new_y > geom.y {
                    let dy = new_y - geom.y;
                    geom.y = new_y;
                    current_offset_y += dy;
                }
            }

            if my_float == crate::engine::style::css_values::CssFloat::Left {
                let left_edge = float_ctx.get_left_offset(geom.y, geom.y + geom.height);
                geom.x = geom.x.max(left_edge);

                float_ctx.left_floats.push(FloatRect {
                    x: geom.x,
                    y: geom.y,
                    width: geom.width,
                    height: geom.height,
                });
            } else if my_float == crate::engine::style::css_values::CssFloat::Right {
                float_ctx.right_floats.push(FloatRect {
                    x: geom.x,
                    y: geom.y,
                    width: geom.width,
                    height: geom.height,
                });
            } else if my_float == crate::engine::style::css_values::CssFloat::None
                && geom.width > 0.0
            {
                let left_edge = float_ctx.get_left_offset(geom.y, geom.y + geom.height);
                if left_edge > geom.x {
                    let dx = left_edge - geom.x;
                    geom.x = left_edge;
                    geom.width = (geom.width - dx).max(0.0);
                }
            }
        }

        for &child_idx in &node.children {
            current_offset_y =
                self.apply_floats_recursive(child_idx, dom, float_ctx, geometry, current_offset_y);
        }

        current_offset_y
    }

    fn build_layout_tree(
        &self,
        dom: &mut AceDOM,
        taffy: &mut taffy::Taffy,
        stylesheet: &Stylesheet,
        node_idx: usize,
        vw: f32,
        vh: f32,
        node_map: &mut std::collections::HashMap<taffy::prelude::Node, usize>,
        parent_grid_ctx: Option<&GridContext>,
    ) -> Vec<taffy::prelude::Node> {
        let (node_type, children_indices) = {
            let node = dom.get_node(node_idx).unwrap();
            (node.node_type.clone(), node.children.clone())
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        let style = {
            let engine_styles = self.element_styles.lock().unwrap();
            engine_styles.get(&node_idx).cloned().unwrap_or_else(|| {
                let am = self.animation_manager.lock().unwrap();
                stylesheet.calculate_style(
                    dom,
                    node_idx,
                    None,
                    None,
                    self.hovered_element,
                    self.focused_element,
                    self.active_element,
                    Some(&am),
                    now,
                    vw,
                    vh,
                    "light",
                )
            })
        };

        // --- Incremental Check ---
        let existing_node = {
            let n2t = self.node_to_taffy.lock().unwrap();
            n2t.get(&node_idx).cloned()
        };

        let node_flags = {
            let node = dom.get_node(node_idx).unwrap();
            node.dirty
        };

        let is_strictly_dirty = node_flags.intersects(
            crate::engine::dom::NodeDirtyFlags::LAYOUT
                | crate::engine::dom::NodeDirtyFlags::CHILDREN
                | crate::engine::dom::NodeDirtyFlags::STYLE,
        );
        let has_dirty_descendants =
            node_flags.contains(crate::engine::dom::NodeDirtyFlags::SUBTREE);

        if let Some(taffy_node) = existing_node {
            if !is_strictly_dirty && !has_dirty_descendants && parent_grid_ctx.is_none() {
                // TRUE INCREMENTAL: Node and its entire subtree are clean. Skip everything.
                node_map.insert(taffy_node, node_idx);
                self.populate_node_map_recursively(dom, taffy, taffy_node, node_idx, node_map);
                return vec![taffy_node];
            }

            if !is_strictly_dirty && has_dirty_descendants && parent_grid_ctx.is_none() {
                // Style/Layout is clean, but children need work.
                // We don't return early here, but we will skip set_style later.
            }
        }

        // --- Element Geometry: Style Extraction ---
        {
            let mut geometry = self.element_geometry.lock().unwrap();
            let geom = geometry.entry(node_idx).or_insert(ElementGeometry::new());

            // Resolve Box Model
            let resolve = |l: &crate::engine::style::css_values::CssLength| -> f32 {
                crate::engine::style::css_values::resolve_length(l, style.font_size, 16.0, vw, vh)
            };

            geom.float = style.float.clone();
            geom.clear = style.clear.clone();

            geom.padding_top = resolve(&style.padding_top);
            geom.padding_right = resolve(&style.padding_right);
            geom.padding_bottom = resolve(&style.padding_bottom);
            geom.padding_left = resolve(&style.padding_left);

            geom.border_top = resolve(&style.border_width_top);
            geom.border_right = resolve(&style.border_width_right);
            geom.border_bottom = resolve(&style.border_width_bottom);
            geom.border_left = resolve(&style.border_width_left);

            geom.overflow_x = style.overflow.clone();
            geom.overflow_y = style.overflow.clone();
        }
        // ------------------------------------------

        let mut taffy_style = self.convert_to_taffy_style(&style);

        // Resolve manual grid placements if in grid container
        if let Some(ctx) = parent_grid_ctx {
            self.apply_grid_context(&mut taffy_style, &style, ctx);
        }

        // Check if we should skip this node (flattening)
        // subgrid also triggers flattening in this implementation to inherit tracks
        let is_subgrid_cols = style
            .grid_template_columns
            .contains(&crate::engine::style::css_values::CssLength::Subgrid);
        let is_subgrid_rows = style
            .grid_template_rows
            .contains(&crate::engine::style::css_values::CssLength::Subgrid);
        let should_flatten = style.display
            == crate::engine::style::css_values::CssDisplay::Contents
            || is_subgrid_cols
            || is_subgrid_rows;

        let mut grid_ctx = None;

        if style.display == crate::engine::style::css_values::CssDisplay::Grid {
            grid_ctx = Some(GridContext {
                column_names: self.resolve_grid_names(&style.grid_template_columns),
                row_names: self.resolve_grid_names(&style.grid_template_rows),
                areas: self.resolve_grid_areas(&style.grid_template_areas),
                col_offset: 0,
                row_offset: 0,
            });
        } else if style.display == crate::engine::style::css_values::CssDisplay::Table {
            // 1. Calculate dimensions and collect cells
            let mut row_count = 0;
            let mut col_count = 0;
            let mut cell_list = Vec::new(); // (row_idx, col_idx, node_idx)

            // Two-Pass Table Layout: Pre-calcular max-width do conteudo de cada coluna.
            // Como Taffy falha com tabelas puras, nós passaremos uma Grid com `px` fixo para CADA track.
            let mut col_max_widths: std::collections::HashMap<usize, f32> =
                std::collections::HashMap::new();

            fn scan_table_children(
                dom: &AceDOM,
                node_idx: usize,
                row_count: &mut usize,
                col_count: &mut usize,
                cell_list: &mut Vec<(usize, usize, usize)>,
                col_max_widths: &mut std::collections::HashMap<usize, f32>,
                font_size_cache: f32,
            ) {
                if let Some(node) = dom.get_node(node_idx) {
                    for &child_idx in &node.children {
                        if let Some(child_node) = dom.get_node(child_idx) {
                            if let crate::engine::dom::AceNodeType::Element(el) =
                                &child_node.node_type
                            {
                                let tag = el.tag.as_str();
                                if tag == "tr" {
                                    let mut current_cols = 0;
                                    for &cell_idx in &child_node.children {
                                        if let Some(cell_node) = dom.get_node(cell_idx) {
                                            if let crate::engine::dom::AceNodeType::Element(
                                                cell_el,
                                            ) = &cell_node.node_type
                                            {
                                                if cell_el.tag == "td" || cell_el.tag == "th" {
                                                    cell_list.push((
                                                        *row_count,
                                                        current_cols,
                                                        cell_idx,
                                                    ));

                                                    // TWO-PASS: Buscar nó de texto filho pra saber a largura bruta em string
                                                    let mut text_len = 0.0;
                                                    for &inner_idx in &cell_node.children {
                                                        if let Some(inner_node) =
                                                            dom.get_node(inner_idx)
                                                        {
                                                            if let crate::engine::dom::AceNodeType::Text(text_str) = &inner_node.node_type {
                                                                 // (Rough character estimation * font_size / 2.0 = text width px)
                                                                 text_len += text_str.len() as f32 * (font_size_cache * 0.55);
                                                            }
                                                        }
                                                    }
                                                    // Add 20px of default padding to the text
                                                    text_len += 20.0;

                                                    let max_w = col_max_widths
                                                        .entry(current_cols)
                                                        .or_insert(0.0);
                                                    if text_len > *max_w {
                                                        *max_w = text_len;
                                                    }

                                                    current_cols += 1;
                                                }
                                            }
                                        }
                                    }
                                    if current_cols > *col_count {
                                        *col_count = current_cols;
                                    }
                                    *row_count += 1;
                                } else if tag == "thead" || tag == "tbody" || tag == "tfoot" {
                                    scan_table_children(
                                        dom,
                                        child_idx,
                                        row_count,
                                        col_count,
                                        cell_list,
                                        col_max_widths,
                                        font_size_cache,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            scan_table_children(
                dom,
                node_idx,
                &mut row_count,
                &mut col_count,
                &mut cell_list,
                &mut col_max_widths,
                style.font_size,
            );

            // 2. Set Grid Template with Pre-calculated PX
            if col_count > 0 {
                let mut tracks = Vec::new();
                for i in 0..col_count {
                    let computed_px = *col_max_widths.get(&i).unwrap_or(&100.0);
                    tracks.push(taffy::prelude::TrackSizingFunction::Single(
                        taffy::geometry::MinMax {
                            min: taffy::prelude::MinTrackSizingFunction::Fixed(
                                taffy::prelude::LengthPercentage::Points(computed_px),
                            ),
                            max: taffy::prelude::MaxTrackSizingFunction::Fixed(
                                taffy::prelude::LengthPercentage::Points(computed_px),
                            ),
                        },
                    ));
                }
                taffy_style.grid_template_columns = tracks;
            }

            // 3. Create Children (Cells flattened)
            let mut children = Vec::new();
            for (r, c, cell_idx) in cell_list {
                let cell_nodes = self
                    .build_layout_tree(dom, taffy, stylesheet, cell_idx, vw, vh, node_map, None);
                if let Some(&cell_taffy_node) = cell_nodes.first() {
                    let mut cell_style = taffy.style(cell_taffy_node).unwrap().clone();
                    // 1-based index for grid placement
                    cell_style.grid_row.start =
                        taffy::prelude::GridPlacement::Line((r as i16 + 1).into());
                    cell_style.grid_column.start =
                        taffy::prelude::GridPlacement::Line((c as i16 + 1).into());
                    let _ = taffy.set_style(cell_taffy_node, cell_style);
                    children.push(cell_taffy_node);
                }
            }

            let taffy_node = taffy.new_with_children(taffy_style, &children).unwrap();
            node_map.insert(taffy_node, node_idx);
            return vec![taffy_node];
        } else if (is_subgrid_cols || is_subgrid_rows) && parent_grid_ctx.is_some() {
            // If this is a subgrid, we use the parent's context but with offsets
            let parent_ctx = parent_grid_ctx.unwrap();
            let mut ctx = parent_ctx.clone();

            let resolve_line_local = |l: &crate::engine::style::css_values::CssLength,
                                      names: &std::collections::HashMap<String, Vec<i16>>|
             -> Option<i16> {
                use crate::engine::style::css_values::CssLength;
                match l {
                    CssLength::Number(v) => Some(*v as i16),
                    CssLength::Name(n) => names.get(n).and_then(|v| v.first().copied()),
                    CssLength::Span(v) => Some(*v as i16),
                    _ => None,
                }
            };

            // Calculate offsets based on subgrid's own placement in parent
            if let Some(start) =
                resolve_line_local(&style.grid_column_start, &parent_ctx.column_names)
            {
                ctx.col_offset += start - 1;
            } else if let crate::engine::style::css_values::CssLength::Name(name) =
                &style.grid_column_start
            {
                if let Some(area) = parent_ctx.areas.get(name) {
                    ctx.col_offset += (area.2 as i16) - 1;
                }
            }

            if let Some(start) = resolve_line_local(&style.grid_row_start, &parent_ctx.row_names) {
                ctx.row_offset += start - 1;
            } else if let crate::engine::style::css_values::CssLength::Name(name) =
                &style.grid_row_start
            {
                if let Some(area) = parent_ctx.areas.get(name) {
                    ctx.row_offset += (area.0 as i16) - 1;
                }
            }

            grid_ctx = Some(ctx);
        } else if let crate::engine::dom::AceNodeType::Element(el) = &node_type {
            if el.tag == "iframe" {
                // If this is an iframe, ensure we have a subframe engine for it
                let mut needs_init = false;
                let mut iframe_src = String::new();

                {
                    if dom.subframes.is_none() {
                        dom.subframes = Some(std::sync::Arc::new(std::sync::Mutex::new(
                            std::collections::HashMap::new(),
                        )));
                    }

                    let subframes_arc = dom.subframes.as_ref().unwrap().clone();
                    let mut subframes = subframes_arc.lock().unwrap();

                    if !subframes.contains_key(&node_idx) {
                        let sub_engine =
                            std::sync::Arc::new(std::sync::Mutex::new(AceEngine::new()));
                        // Marcar qual nó <iframe> este subframe representa no pai
                        {
                            let sub_eng = sub_engine.lock().unwrap();
                            if let Some(ref dom_arc) = sub_eng.dom {
                                let mut sub_dom = dom_arc.lock().unwrap();
                                sub_dom.iframe_node_idx = Some(node_idx);
                            }
                        }
                        subframes.insert(node_idx, sub_engine);
                        needs_init = true;

                        if let Some(src) = el.attributes.get("src") {
                            iframe_src = src.clone();
                        }
                    }
                }

                if needs_init && !iframe_src.is_empty() {
                    if let Some(subframes_arc) = &dom.subframes {
                        let subframes_lock = subframes_arc.lock().unwrap();
                        if let Some(engine_arc) = subframes_lock.get(&node_idx) {
                            let mut sub_engine = engine_arc.lock().unwrap();
                            // Copy over resource manager and store target URL.
                            // JS runtime is initialized lazily when the iframe content
                            // is actually loaded (do NOT call init_js_for_url here as
                            // it runs init_stdlib which uses tokio and may block).
                            sub_engine.resource_manager = self.resource_manager.clone();
                            sub_engine.current_url = iframe_src.clone();
                        }
                    }
                }
            } else if el.tag == "svg" {
                // SVGs are treated as leaf nodes in layout, but we need to extract their dimensions
                let mut svg_width = style.width.clone();
                let mut svg_height = style.height.clone();

                // If CSS size is auto, try to get from attributes
                if svg_width == crate::engine::style::css_values::CssLength::Auto {
                    if let Some(w_attr) = el.attributes.get("width") {
                        if let Ok(w) = w_attr.parse::<f32>() {
                            svg_width = crate::engine::style::css_values::CssLength::Px(w);
                        }
                    }
                }
                if svg_height == crate::engine::style::css_values::CssLength::Auto {
                    if let Some(h_attr) = el.attributes.get("height") {
                        if let Ok(h) = h_attr.parse::<f32>() {
                            svg_height = crate::engine::style::css_values::CssLength::Px(h);
                        }
                    }
                }

                // Override taffy style size for the SVG leaf node
                taffy_style.size.width = self.to_taffy_dimension(&svg_width);
                taffy_style.size.height = self.to_taffy_dimension(&svg_height);

                let taffy_node = taffy.new_leaf(taffy_style).unwrap();
                node_map.insert(taffy_node, node_idx);
                return vec![taffy_node];
            }
        } else if let crate::engine::dom::AceNodeType::Text(text) = &node_type {
            let transformed_text =
                crate::engine::inline::apply_text_transform(text.as_ref(), &style.text_transform);
            // Text nodes need an intrinsic size estimate to be visible
            let font_size = style.font_size;
            let line_height = font_size * 1.2;

            // Usar o TextMeasurer global para dimensões reais
            let letter_spacing = crate::engine::style::css_values::resolve_length(
                &style.letter_spacing,
                font_size,
                16.0,
                vw,
                vh,
            );
            let word_spacing = crate::engine::style::css_values::resolve_length(
                &style.word_spacing,
                font_size,
                16.0,
                vw,
                vh,
            );

            let (text_width, text_height) = self.text_measurer.measure_text(
                &transformed_text,
                font_size,
                line_height,
                Some(&style.font_family),
                cosmic_text::Weight::NORMAL,
                None,
                letter_spacing,
                word_spacing,
            );

            let width = text_width;
            let height = text_height;

            taffy_style.size.width = taffy::prelude::Dimension::Points(width);
            taffy_style.size.height = taffy::prelude::Dimension::Points(height);

            // Se o texto for curto, não queremos que ele "estique" se for um bloco
            taffy_style.max_size.width = taffy::prelude::Dimension::Points(width);

            let taffy_node = if let Some(node) = existing_node {
                if is_strictly_dirty {
                    let _ = taffy.set_style(node, taffy_style);
                }
                node
            } else {
                let node = taffy.new_leaf(taffy_style).unwrap();
                self.node_to_taffy.lock().unwrap().insert(node_idx, node);
                node
            };

            node_map.insert(taffy_node, node_idx);
            return vec![taffy_node];
        }

        let mut children = Vec::new();

        if style.display == crate::engine::style::css_values::CssDisplay::Table {
            // Already handled in build_layout_tree main block for tables
        } else if style.display == crate::engine::style::css_values::CssDisplay::TableRow {
            // Table rows are flattened, their children (cells) become direct children of the table grid
            for child_idx in children_indices {
                children.extend(self.build_layout_tree(
                    dom,
                    taffy,
                    stylesheet,
                    child_idx,
                    vw,
                    vh,
                    node_map,
                    grid_ctx.as_ref(),
                ));
            }
        } else if style.display == crate::engine::style::css_values::CssDisplay::TableHeader {
            // Table headers are also flattened, their children (cells) become direct children of the table grid
            for child_idx in children_indices {
                children.extend(self.build_layout_tree(
                    dom,
                    taffy,
                    stylesheet,
                    child_idx,
                    vw,
                    vh,
                    node_map,
                    grid_ctx.as_ref(),
                ));
            }
        } else {
            // <details> filtering: quando fechado (sem atributo "open"),
            // renderizar apenas filhos <summary>, ignorando todo o resto
            let is_details_closed = if let crate::engine::dom::AceNodeType::Element(el) = &node_type
            {
                el.tag == "details" && !el.attributes.contains_key("open")
            } else {
                false
            };

            for child_idx in children_indices {
                if is_details_closed {
                    if let Some(child_node) = dom.get_node(child_idx) {
                        let is_summary = if let crate::engine::dom::AceNodeType::Element(child_el) =
                            &child_node.node_type
                        {
                            child_el.tag == "summary"
                        } else {
                            false
                        };
                        if !is_summary {
                            continue;
                        }
                    }
                }
                children.extend(self.build_layout_tree(
                    dom,
                    taffy,
                    stylesheet,
                    child_idx,
                    vw,
                    vh,
                    node_map,
                    grid_ctx.as_ref(),
                ));
            }
        }

        if should_flatten {
            // If flattening, we don't create a Taffy node for THIS element.
            // We just return its children to be added to the grandparent.
            return children;
        }

        let taffy_node = if let Some(node) = existing_node {
            if is_strictly_dirty {
                let _ = taffy.set_style(node, taffy_style);
            }
            // Only update children if list is potentially changed
            let is_children_dirty = {
                let dom_node = dom.get_node(node_idx).unwrap();
                dom_node.dirty.intersects(
                    crate::engine::dom::NodeDirtyFlags::CHILDREN
                        | crate::engine::dom::NodeDirtyFlags::LAYOUT,
                )
            };
            if is_children_dirty || taffy.children(node).unwrap_or_default().len() != children.len()
            {
                let _ = taffy.set_children(node, &children);
            }
            node
        } else {
            let node = taffy.new_with_children(taffy_style, &children).unwrap();
            self.node_to_taffy.lock().unwrap().insert(node_idx, node);
            node
        };

        node_map.insert(taffy_node, node_idx);
        vec![taffy_node]
    }

    fn populate_node_map_recursively(
        &self,
        dom: &AceDOM,
        taffy: &taffy::Taffy,
        node: taffy::prelude::Node,
        node_idx: usize,
        node_map: &mut std::collections::HashMap<taffy::prelude::Node, usize>,
    ) {
        node_map.insert(node, node_idx);
        if let Ok(children) = taffy.children(node) {
            if let Some(dom_node) = dom.get_node(node_idx) {
                // Simplified 1:1 mapping for stable subtrees
                for (i, &taffy_child) in children.iter().enumerate() {
                    if i < dom_node.children.len() {
                        self.populate_node_map_recursively(
                            dom,
                            taffy,
                            taffy_child,
                            dom_node.children[i],
                            node_map,
                        );
                    }
                }
            }
        }
    }

    fn apply_grid_context(
        &self,
        t_style: &mut taffy::prelude::Style,
        style: &crate::engine::style::css_values::ComputedStyle,
        ctx: &GridContext,
    ) {
        use crate::engine::style::css_values::CssLength;

        let resolve_line =
            |l: &CssLength, names: &std::collections::HashMap<String, Vec<i16>>| -> Option<i16> {
                match l {
                    CssLength::Number(v) => Some(*v as i16),
                    CssLength::Name(n) => names.get(n).and_then(|v| v.first().copied()),
                    _ => None,
                }
            };

        if let Some(start) = resolve_line(&style.grid_column_start, &ctx.column_names) {
            t_style.grid_column.start =
                taffy::prelude::GridPlacement::Line((start + ctx.col_offset).into());
        }
        if let Some(end) = resolve_line(&style.grid_column_end, &ctx.column_names) {
            t_style.grid_column.end =
                taffy::prelude::GridPlacement::Line((end + ctx.col_offset).into());
        }
        if let Some(start) = resolve_line(&style.grid_row_start, &ctx.row_names) {
            t_style.grid_row.start =
                taffy::prelude::GridPlacement::Line((start + ctx.row_offset).into());
        }
        if let Some(end) = resolve_line(&style.grid_row_end, &ctx.row_names) {
            t_style.grid_row.end =
                taffy::prelude::GridPlacement::Line((end + ctx.row_offset).into());
        }

        // Apply grid-area name if it matches an area
        if let CssLength::Name(name) = &style.grid_row_start {
            if let Some(area) = ctx.areas.get(name) {
                t_style.grid_row.start =
                    taffy::prelude::GridPlacement::Line(((area.0 as i16) + ctx.row_offset).into());
                t_style.grid_row.end =
                    taffy::prelude::GridPlacement::Line(((area.1 as i16) + ctx.row_offset).into());
                t_style.grid_column.start =
                    taffy::prelude::GridPlacement::Line(((area.2 as i16) + ctx.col_offset).into());
                t_style.grid_column.end =
                    taffy::prelude::GridPlacement::Line(((area.3 as i16) + ctx.col_offset).into());
            }
        }
    }

    fn sync_taffy_bounds(
        &self,
        taffy: &taffy::Taffy,
        dom: &AceDOM,
        taffy_node: taffy::prelude::Node,
        offset_x: f32,
        offset_y: f32,
        node_map: &std::collections::HashMap<taffy::prelude::Node, usize>,
    ) {
        let layout = taffy.layout(taffy_node).unwrap();
        let abs_x = offset_x + layout.location.x;
        let abs_y = offset_y + layout.location.y;
        let width = layout.size.width;
        let height = layout.size.height;

        let mut content_w = 0.0f32;
        let mut content_h = 0.0f32;

        // Calculate content dimensions from children
        let taffy_children = taffy.children(taffy_node).unwrap();
        if !taffy_children.is_empty() {
            for &child in &taffy_children {
                if let Ok(child_layout) = taffy.layout(child) {
                    let right = child_layout.location.x + child_layout.size.width;
                    let bottom = child_layout.location.y + child_layout.size.height;
                    if right > content_w {
                        content_w = right;
                    }
                    if bottom > content_h {
                        content_h = bottom;
                    }
                }
            }
        } else {
            // For leaf nodes (like text), we might need intrinsic size?
            // But Taffy layout size is usually enough for flow content.
        }

        if let Some(&node_idx) = node_map.get(&taffy_node) {
            let mut geometry = self.element_geometry.lock().unwrap();
            // Ensure entry exists (it should from build_layout_tree)
            let geom = geometry.entry(node_idx).or_insert(ElementGeometry::new());

            geom.x = abs_x;
            geom.y = abs_y;
            geom.width = width;
            geom.height = height;
            geom.content_width = content_w; // This is raw content size relative to padding box
            geom.content_height = content_h;
        }

        // Aplicar offset de scroll interno para os filhos
        let (sx, sy) = if let Some(&node_idx) = node_map.get(&taffy_node) {
            let scroll_map = self.element_scroll.lock().unwrap();
            scroll_map.get(&node_idx).copied().unwrap_or((0.0, 0.0))
        } else {
            (0.0, 0.0)
        };

        for &taffy_child in &taffy_children {
            self.sync_taffy_bounds(taffy, dom, taffy_child, abs_x - sx, abs_y - sy, node_map);
        }
    }

    fn convert_to_taffy_style(
        &self,
        style: &crate::engine::style::css_values::ComputedStyle,
    ) -> taffy::prelude::Style {
        let mut t_style = taffy::prelude::Style::default();

        t_style.display = match style.display {
            crate::engine::style::css_values::CssDisplay::Grid
            | crate::engine::style::css_values::CssDisplay::Table => taffy::prelude::Display::Grid,
            crate::engine::style::css_values::CssDisplay::Flex => taffy::prelude::Display::Flex,
            crate::engine::style::css_values::CssDisplay::None => taffy::prelude::Display::None,
            crate::engine::style::css_values::CssDisplay::Contents
            | crate::engine::style::css_values::CssDisplay::TableRow
            | crate::engine::style::css_values::CssDisplay::TableHeader => {
                taffy::prelude::Display::None
            } // Will be flattened/handled manually
            _ => taffy::prelude::Display::Flex,
        };

        t_style.flex_direction = match style.display {
            crate::engine::style::css_values::CssDisplay::Block => {
                taffy::prelude::FlexDirection::Column
            }
            _ => match style.flex_direction {
                crate::engine::style::css_values::CssFlexDirection::Row => {
                    taffy::prelude::FlexDirection::Row
                }
                crate::engine::style::css_values::CssFlexDirection::Column => {
                    taffy::prelude::FlexDirection::Column
                }
                crate::engine::style::css_values::CssFlexDirection::RowReverse => {
                    taffy::prelude::FlexDirection::RowReverse
                }
                crate::engine::style::css_values::CssFlexDirection::ColumnReverse => {
                    taffy::prelude::FlexDirection::ColumnReverse
                }
            },
        };

        t_style.flex_wrap = match style.flex_wrap {
            crate::engine::style::css_values::CssFlexWrap::NoWrap => {
                taffy::prelude::FlexWrap::NoWrap
            }
            crate::engine::style::css_values::CssFlexWrap::Wrap => taffy::prelude::FlexWrap::Wrap,
            crate::engine::style::css_values::CssFlexWrap::WrapReverse => {
                taffy::prelude::FlexWrap::WrapReverse
            }
        };

        t_style.position = match style.position {
            crate::engine::style::css_values::CssPosition::Absolute
            | crate::engine::style::css_values::CssPosition::Fixed => {
                taffy::prelude::Position::Absolute
            }
            _ => {
                if style.float != crate::engine::style::css_values::CssFloat::None {
                    taffy::prelude::Position::Absolute
                } else {
                    taffy::prelude::Position::Relative
                }
            }
        };

        t_style.size = taffy::prelude::Size {
            width: self.to_taffy_dimension(&style.width),
            height: self.to_taffy_dimension(&style.height),
        };

        t_style.min_size = taffy::prelude::Size {
            width: self.to_taffy_dimension(&style.min_width),
            height: self.to_taffy_dimension(&style.min_height),
        };

        t_style.max_size = taffy::prelude::Size {
            width: self.to_taffy_dimension(&style.max_width),
            height: self.to_taffy_dimension(&style.max_height),
        };

        t_style.margin = taffy::prelude::Rect {
            left: self.to_taffy_length_percentage(&style.margin_left).into(),
            right: self.to_taffy_length_percentage(&style.margin_right).into(),
            top: self.to_taffy_length_percentage(&style.margin_top).into(),
            bottom: self.to_taffy_length_percentage(&style.margin_bottom).into(),
        };

        t_style.padding = taffy::prelude::Rect {
            left: self.to_taffy_length_percentage(&style.padding_left).into(),
            right: self.to_taffy_length_percentage(&style.padding_right).into(),
            top: self.to_taffy_length_percentage(&style.padding_top).into(),
            bottom: self
                .to_taffy_length_percentage(&style.padding_bottom)
                .into(),
        };

        t_style.gap = taffy::prelude::Size {
            width: self.to_taffy_length_percentage(&style.grid_column_gap),
            height: self.to_taffy_length_percentage(&style.grid_row_gap),
        };

        // Filter out LineNames before passing to Taffy
        t_style.grid_template_columns = style
            .grid_template_columns
            .iter()
            .filter(|l| !matches!(l, crate::engine::style::css_values::CssLength::LineNames(_)))
            .map(|l| self.to_taffy_track_size(l))
            .collect();

        t_style.grid_template_rows = style
            .grid_template_rows
            .iter()
            .filter(|l| !matches!(l, crate::engine::style::css_values::CssLength::LineNames(_)))
            .map(|l| self.to_taffy_track_size(l))
            .collect();

        // Resolve Placements
        t_style.grid_column.start = self.resolve_placement(&style.grid_column_start);
        t_style.grid_column.end = self.resolve_placement(&style.grid_column_end);
        t_style.grid_row.start = self.resolve_placement(&style.grid_row_start);
        t_style.grid_row.end = self.resolve_placement(&style.grid_row_end);

        t_style
    }

    fn resolve_placement(
        &self,
        placement: &crate::engine::style::css_values::CssLength,
    ) -> taffy::prelude::GridPlacement {
        use crate::engine::style::css_values::CssLength;
        match placement {
            CssLength::Number(v) => taffy::prelude::GridPlacement::Line((*v as i16).into()),
            CssLength::Span(v) => taffy::prelude::GridPlacement::Span(*v),
            CssLength::Name(_n) => {
                // This is a placeholder; real resolution happens in build_layout_tree
                // if we have parent context.
                taffy::prelude::GridPlacement::Auto
            }
            _ => taffy::prelude::GridPlacement::Auto,
        }
    }

    fn resolve_grid_areas(
        &self,
        areas: &[String],
    ) -> std::collections::HashMap<String, (usize, usize, usize, usize)> {
        let mut map = std::collections::HashMap::new();
        for (row_idx, row_str) in areas.iter().enumerate() {
            let cols: Vec<&str> = row_str.split_whitespace().collect();
            for (col_idx, area_name) in cols.iter().enumerate() {
                if *area_name == "." {
                    continue;
                }
                let entry = map.entry(area_name.to_string()).or_insert((
                    row_idx,
                    row_idx + 1,
                    col_idx,
                    col_idx + 1,
                ));
                entry.0 = entry.0.min(row_idx);
                entry.1 = entry.1.max(row_idx + 1);
                entry.2 = entry.2.min(col_idx);
                entry.3 = entry.3.max(col_idx + 1);
            }
        }
        map
    }

    fn resolve_grid_names(
        &self,
        tracks: &[crate::engine::style::css_values::CssLength],
    ) -> std::collections::HashMap<String, Vec<i16>> {
        let mut map = std::collections::HashMap::new();
        let mut line_idx = 0;
        for track in tracks {
            match track {
                crate::engine::style::css_values::CssLength::LineNames(names) => {
                    for name in names {
                        map.entry(name.clone()).or_insert(Vec::new()).push(line_idx);
                    }
                }
                _ => {
                    line_idx += 1;
                }
            }
        }
        map
    }

    fn to_taffy_dimension(
        &self,
        len: &crate::engine::style::css_values::CssLength,
    ) -> taffy::prelude::Dimension {
        use crate::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::Dimension::Points(*v),
            CssLength::Percent(v) => taffy::prelude::Dimension::Percent(*v / 100.0),
            CssLength::Auto => taffy::prelude::Dimension::Auto,
            _ => taffy::prelude::Dimension::Auto,
        }
    }

    fn to_taffy_length_percentage_auto(
        &self,
        len: &crate::engine::style::css_values::CssLength,
    ) -> taffy::prelude::LengthPercentageAuto {
        use crate::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::LengthPercentageAuto::Points(*v),
            CssLength::Percent(v) => taffy::prelude::LengthPercentageAuto::Percent(*v / 100.0),
            CssLength::Auto => taffy::prelude::LengthPercentageAuto::Auto,
            _ => taffy::prelude::LengthPercentageAuto::Points(0.0),
        }
    }

    fn to_taffy_length_percentage(
        &self,
        len: &crate::engine::style::css_values::CssLength,
    ) -> taffy::prelude::LengthPercentage {
        use crate::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::LengthPercentage::Points(*v),
            CssLength::Percent(v) => taffy::prelude::LengthPercentage::Percent(*v / 100.0),
            _ => taffy::prelude::LengthPercentage::Points(0.0),
        }
    }

    fn to_taffy_track_size(
        &self,
        len: &crate::engine::style::css_values::CssLength,
    ) -> taffy::prelude::TrackSizingFunction {
        use crate::engine::style::css_values::CssLength;
        match len {
            CssLength::Fr(v) => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: taffy::prelude::MinTrackSizingFunction::Auto,
                max: taffy::prelude::MaxTrackSizingFunction::Fraction(*v),
            }),
            CssLength::Px(v) => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: taffy::prelude::MinTrackSizingFunction::Fixed(
                    taffy::prelude::LengthPercentage::Points(*v),
                ),
                max: taffy::prelude::MaxTrackSizingFunction::Fixed(
                    taffy::prelude::LengthPercentage::Points(*v),
                ),
            }),
            CssLength::Percent(v) => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: taffy::prelude::MinTrackSizingFunction::Fixed(
                    taffy::prelude::LengthPercentage::Percent(*v / 100.0),
                ),
                max: taffy::prelude::MaxTrackSizingFunction::Fixed(
                    taffy::prelude::LengthPercentage::Percent(*v / 100.0),
                ),
            }),
            CssLength::MinMax(min, max) => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: self.to_taffy_min_track(min),
                max: self.to_taffy_max_track(max),
            }),
            CssLength::Repeat(count, sub) => {
                let repetition = match count.as_str() {
                    "auto-fill" => taffy::prelude::GridTrackRepetition::AutoFill,
                    "auto-fit" => taffy::prelude::GridTrackRepetition::AutoFit,
                    _ => {
                        let c = count.parse::<u16>().unwrap_or(1);
                        taffy::prelude::GridTrackRepetition::Count(c)
                    }
                };
                let tracks = sub
                    .iter()
                    .map(|l| match self.to_taffy_track_size(l) {
                        taffy::prelude::TrackSizingFunction::Single(mm) => mm,
                        _ => MinMax {
                            min: taffy::prelude::MinTrackSizingFunction::Auto,
                            max: taffy::prelude::MaxTrackSizingFunction::Auto,
                        },
                    })
                    .collect();
                taffy::prelude::TrackSizingFunction::Repeat(repetition, tracks)
            }
            _ => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: taffy::prelude::MinTrackSizingFunction::Auto,
                max: taffy::prelude::MaxTrackSizingFunction::Auto,
            }),
        }
    }

    fn to_taffy_min_track(
        &self,
        len: &crate::engine::style::css_values::CssLength,
    ) -> taffy::prelude::MinTrackSizingFunction {
        use crate::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::MinTrackSizingFunction::Fixed(
                taffy::prelude::LengthPercentage::Points(*v),
            ),
            CssLength::Percent(v) => taffy::prelude::MinTrackSizingFunction::Fixed(
                taffy::prelude::LengthPercentage::Percent(*v / 100.0),
            ),
            CssLength::MinContent => taffy::prelude::MinTrackSizingFunction::MinContent,
            CssLength::MaxContent => taffy::prelude::MinTrackSizingFunction::MaxContent,
            _ => taffy::prelude::MinTrackSizingFunction::Auto,
        }
    }

    fn to_taffy_max_track(
        &self,
        len: &crate::engine::style::css_values::CssLength,
    ) -> taffy::prelude::MaxTrackSizingFunction {
        use crate::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::MaxTrackSizingFunction::Fixed(
                taffy::prelude::LengthPercentage::Points(*v),
            ),
            CssLength::Percent(v) => taffy::prelude::MaxTrackSizingFunction::Fixed(
                taffy::prelude::LengthPercentage::Percent(*v / 100.0),
            ),
            CssLength::Fr(v) => taffy::prelude::MaxTrackSizingFunction::Fraction(*v),
            CssLength::MinContent => taffy::prelude::MaxTrackSizingFunction::MinContent,
            CssLength::MaxContent => taffy::prelude::MaxTrackSizingFunction::MaxContent,
            _ => taffy::prelude::MaxTrackSizingFunction::Auto,
        }
    }

    pub fn css_color_to_skia(
        css_color: &crate::engine::style::css_values::CssColor,
    ) -> Option<tiny_skia::Color> {
        use crate::engine::style::css_values::CssColor;
        match css_color {
            CssColor::Rgba(r, g, b, a) => {
                Some(tiny_skia::Color::from_rgba8(*r, *g, *b, (*a * 255.0) as u8))
            }
            CssColor::Named(name) => {
                if name.eq_ignore_ascii_case("transparent") {
                    return None;
                }
                if name.starts_with("#") {
                    let hex = name.trim_start_matches('#');
                    if hex.len() == 3 {
                        // Expandir dígito hex: 0xA => 0xAA == A * 17 (zero alocações)
                        let r = u8::from_str_radix(&hex[0..1], 16).unwrap_or(0) * 17;
                        let g = u8::from_str_radix(&hex[1..2], 16).unwrap_or(0) * 17;
                        let b = u8::from_str_radix(&hex[2..3], 16).unwrap_or(0) * 17;
                        Some(tiny_skia::Color::from_rgba8(r, g, b, 255))
                    } else {
                        let r =
                            u8::from_str_radix(if hex.len() >= 2 { &hex[0..2] } else { hex }, 16)
                                .unwrap_or(0);
                        let g =
                            u8::from_str_radix(if hex.len() >= 4 { &hex[2..4] } else { "0" }, 16)
                                .unwrap_or(0);
                        let b =
                            u8::from_str_radix(if hex.len() >= 6 { &hex[4..6] } else { "0" }, 16)
                                .unwrap_or(0);
                        let a = if hex.len() == 8 {
                            u8::from_str_radix(&hex[6..8], 16).unwrap_or(255)
                        } else {
                            255
                        };
                        Some(tiny_skia::Color::from_rgba8(r, g, b, a))
                    }
                } else {
                    let lower = name.to_ascii_lowercase();
                    match lower.as_str() {
                        "white" => Some(tiny_skia::Color::from_rgba8(255, 255, 255, 255)),
                        "black" | "currentcolor" => {
                            Some(tiny_skia::Color::from_rgba8(0, 0, 0, 255))
                        }
                        "red" => Some(tiny_skia::Color::from_rgba8(255, 0, 0, 255)),
                        "green" => Some(tiny_skia::Color::from_rgba8(0, 128, 0, 255)),
                        "blue" => Some(tiny_skia::Color::from_rgba8(0, 0, 255, 255)),
                        "darkblue" => Some(tiny_skia::Color::from_rgba8(0, 0, 139, 255)),
                        "darkgreen" => Some(tiny_skia::Color::from_rgba8(0, 100, 0, 255)),
                        "darkred" => Some(tiny_skia::Color::from_rgba8(139, 0, 0, 255)),
                        "yellow" => Some(tiny_skia::Color::from_rgba8(255, 255, 0, 255)),
                        "gray" | "grey" => Some(tiny_skia::Color::from_rgba8(128, 128, 128, 255)),
                        "lightslategray" => Some(tiny_skia::Color::from_rgba8(119, 136, 153, 255)),
                        _ => None,
                    }
                }
            }
            _ => None,
        }
    }

    pub fn render_visual(&self, vw: f32, vh: f32) -> crate::engine::layer_tree::LayerTree {
        let mut items = Vec::new();
        let mut fixed_nodes = Vec::new();

        // ─── VIRTUAL SCROLLING / CULLING BOUNDARIES ─────────────
        // viewport_y is negative when scrolled down
        let scroll_y = (-self.viewport_y).max(0.0);
        // Add a buffer so elements don't pop-in instantly
        let culling_buffer = vh * 1.5;
        let visible_top = scroll_y - culling_buffer;
        let visible_bottom = scroll_y + vh + culling_buffer;
        // ────────────────────────────────────────────────────────

        if let Some(ref dom_arc) = self.dom {
            let dom = dom_arc.lock().unwrap();
            let stylesheet = self.stylesheet.lock().unwrap();
            let geometry = self.element_geometry.lock().unwrap();
            let element_styles = self.element_styles.lock().unwrap();

            for (node_idx, node) in dom.nodes.iter().enumerate() {
                let (is_element, element_tag): (bool, &str) = match &node.node_type {
                    crate::engine::dom::AceNodeType::Element(el) => (true, el.tag.as_str()),
                    crate::engine::dom::AceNodeType::Text(_) => (false, "#text"),
                    _ => continue,
                };

                // Ocultar tags técnicas e seu conteúdo (CSS, JS, etc.)
                if is_element && self.is_technical_tag(&element_tag) {
                    continue;
                }
                if self.has_technical_ancestor(&dom, node_idx) {
                    continue;
                }

                // Usar estilo cacheado ou calcular se faltar (ex: novos nós)
                let mut computed_style = if let Some(cached) = element_styles.get(&node_idx) {
                    cached.clone()
                } else {
                    // Fallback para calculate_style simples (ponto de melhoria futuro: herança aqui também)
                    stylesheet.calculate_style(
                        &dom,
                        node_idx,
                        None,
                        None,
                        self.hovered_element,
                        self.focused_element,
                        self.active_element,
                        None,
                        0.0,
                        vw,
                        vh,
                        "light",
                    )
                };

                // Inherit parent styles for pure text nodes
                if !is_element {
                    if let Some(parent_idx) = node.parent {
                        if let Some(parent_style) = element_styles.get(&parent_idx) {
                            computed_style = parent_style.clone();
                        }
                    }
                }

                if matches!(
                    computed_style.display,
                    crate::engine::style::css_values::CssDisplay::None
                ) {
                    continue;
                }

                if let Some(geom) = geometry.get(&node_idx) {
                    let x = geom.x;
                    let mut y = geom.y;
                    let w = geom.width;
                    let h = geom.height;
                    let mut is_sticky_fixed = false;

                    // ─── STICKY POSITIONING ─────────────────────────────────
                    // position:sticky → o elemento flui normalmente até que o
                    // scroll do viewport ultrapasse seu threshold (top/bottom).
                    // Quando "stuck", comporta-se como fixed, clampado pelo parent.
                    let effective_position = if is_element {
                        computed_style.position.clone()
                    } else {
                        // Texto herda sticky do pai
                        if let Some(parent_idx) = node.parent {
                            if let Some(ps) = element_styles.get(&parent_idx) {
                                ps.position.clone()
                            } else {
                                crate::engine::style::css_values::CssPosition::Static
                            }
                        } else {
                            crate::engine::style::css_values::CssPosition::Static
                        }
                    };

                    if effective_position == crate::engine::style::css_values::CssPosition::Sticky {
                        let font_size = computed_style.font_size;
                        // Resolver o threshold CSS (top, bottom)
                        let sticky_top = crate::engine::style::css_values::resolve_length(
                            &computed_style.top,
                            font_size,
                            16.0,
                            vw,
                            vh,
                        );

                        // scroll_y: quanto o viewport desceu (viewport_y é negativo quando scrollado)
                        let scroll_y = (-self.viewport_y).max(0.0);

                        // Para texto filho de sticky, usar a geometria do pai sticky
                        let (natural_y, elem_h, parent_info) = if !is_element {
                            if let Some(parent_idx) = node.parent {
                                if let Some(pg) = geometry.get(&parent_idx) {
                                    // Delta relativo ao pai
                                    let delta_y = geom.y - pg.y;
                                    // Buscar avô para constraint
                                    let gp_info = if let Some(pnode) = dom.get_node(parent_idx) {
                                        if let Some(gp_idx) = pnode.parent {
                                            geometry.get(&gp_idx).map(|gpg| (gpg.y, gpg.height))
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };
                                    (pg.y, pg.height, Some((delta_y, gp_info)))
                                } else {
                                    (geom.y, h, None)
                                }
                            } else {
                                (geom.y, h, None)
                            }
                        } else {
                            (geom.y, h, None)
                        };

                        // Buscar parent container para clampar
                        let (_parent_top, parent_bottom) = if let Some((_, gp_info)) = &parent_info
                        {
                            // Para texto: constraint é o avô do sticky
                            if let Some((gp_y, gp_h)) = gp_info {
                                (*gp_y, *gp_y + *gp_h)
                            } else {
                                (0.0, vh * 10.0) // fallback
                            }
                        } else if let Some(parent_idx) = node.parent {
                            if let Some(pg) = geometry.get(&parent_idx) {
                                (pg.y, pg.y + pg.height)
                            } else {
                                (0.0, vh * 10.0)
                            }
                        } else {
                            (0.0, vh * 10.0)
                        };

                        // Clampar: o elemento gruda quando sairia do viewport
                        let threshold_line = scroll_y + sticky_top;
                        if natural_y < threshold_line {
                            // Elemento grudou — fixar na posição do threshold
                            let max_y = parent_bottom - elem_h;
                            let stuck_y = threshold_line.min(max_y);

                            if let Some((delta_y, _)) = parent_info {
                                // Texto filho: aplicar o mesmo delta do pai sticky
                                y = stuck_y + delta_y;
                            } else {
                                y = stuck_y;
                            }
                            is_sticky_fixed = true;
                        }
                    }
                    // ─── FIM STICKY ─────────────────────────────────────────

                    // ─── CULLING (VIRTUAL SCROLLING) ────────────────────────
                    // Skip processing elements completely outside the visible viewport
                    // Exceptions: position:fixed (always on screen)
                    if effective_position != crate::engine::style::css_values::CssPosition::Fixed {
                        let element_top = y;
                        let element_bottom = y + h;

                        if element_bottom < visible_top || element_top > visible_bottom {
                            continue; // Element is entirely culled
                        }
                    }
                    // ────────────────────────────────────────────────────────

                    let background_color =
                        Self::css_color_to_skia(&computed_style.background_color);
                    let border_color = Self::css_color_to_skia(&computed_style.border_color_top);
                    let text_color = Self::css_color_to_skia(&computed_style.color)
                        .unwrap_or(tiny_skia::Color::BLACK);

                    let mut text = match &node.node_type {
                        crate::engine::dom::AceNodeType::Text(t) => {
                            crate::engine::inline::apply_text_transform(
                                t.as_ref(),
                                &computed_style.text_transform,
                            )
                        }
                        _ => String::new(),
                    };

                    // Disclosure marker para <summary>: ▶ (fechado) ou ▼ (aberto)
                    if !text.is_empty() {
                        if let Some(parent_idx) = node.parent {
                            if let Some(parent_node) = dom.get_node(parent_idx) {
                                if let crate::engine::dom::AceNodeType::Element(parent_el) =
                                    &parent_node.node_type
                                {
                                    if parent_el.tag == "summary" {
                                        // Verificar se este é o primeiro filho de texto do summary
                                        let is_first_text =
                                            parent_node.children.first() == Some(&node_idx);
                                        if is_first_text {
                                            // Verificar o avô <details> pelo atributo "open"
                                            let is_open = if let Some(gp_idx) = parent_node.parent {
                                                if let Some(gp_node) = dom.get_node(gp_idx) {
                                                    if let crate::engine::dom::AceNodeType::Element(gp_el) = &gp_node.node_type {
                                                        gp_el.tag == "details" && gp_el.attributes.contains_key("open")
                                                    } else { false }
                                                } else {
                                                    false
                                                }
                                            } else {
                                                false
                                            };
                                            let marker = if is_open { "▼ " } else { "▶ " };
                                            text = format!("{}{}", marker, text);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // --- OPTIMIZED TEXT-OVERFLOW: ELLIPSIS (AceEngine) ---
                    // This logic handles the case where a single text node overflows its container
                    // specifically with white-space: nowrap and text-overflow: ellipsis/clip.
                    if !text.is_empty()
                        && matches!(
                            computed_style.white_space,
                            crate::engine::style::css_values::CssWhiteSpace::NoWrap
                        )
                    {
                        let font_size = computed_style.font_size;
                        let line_height = crate::engine::style::css_values::resolve_length(
                            &computed_style.line_height,
                            font_size,
                            16.0,
                            vw,
                            vh,
                        );
                        let line_height = if line_height <= 0.0 {
                            font_size * 1.2
                        } else {
                            line_height
                        };

                        let letter_spacing = crate::engine::style::css_values::resolve_length(
                            &computed_style.letter_spacing,
                            font_size,
                            16.0,
                            vw,
                            vh,
                        );
                        let word_spacing = crate::engine::style::css_values::resolve_length(
                            &computed_style.word_spacing,
                            font_size,
                            16.0,
                            vw,
                            vh,
                        );

                        let (total_w, _) = self.text_measurer.measure_text(
                            &text,
                            font_size,
                            line_height,
                            Some(&computed_style.font_family),
                            cosmic_text::Weight::NORMAL,
                            None,
                            letter_spacing,
                            word_spacing,
                        );

                        if total_w > w {
                            let use_ellipsis = matches!(
                                computed_style.text_overflow,
                                crate::engine::style::css_values::CssTextOverflow::Ellipsis
                            );
                            let ellipsis = "…";
                            let (ell_w, _) = if use_ellipsis {
                                self.text_measurer.measure_text(
                                    ellipsis,
                                    font_size,
                                    line_height,
                                    Some(&computed_style.font_family),
                                    cosmic_text::Weight::NORMAL,
                                    None,
                                    letter_spacing,
                                    word_spacing,
                                )
                            } else {
                                (0.0, 0.0)
                            };

                            let safe_width = w - ell_w;

                            if safe_width > 0.0 {
                                let mut best_len = 0;
                                let mut left = 0;
                                let mut right = text.len();

                                while left <= right {
                                    let mid: usize = (left + right) / 2;
                                    let mut mid_adj = mid;
                                    while mid_adj > 0 && !text.is_char_boundary(mid_adj) {
                                        mid_adj -= 1;
                                    }

                                    let (sub_w, _) = self.text_measurer.measure_text(
                                        &text[..mid_adj],
                                        font_size,
                                        line_height,
                                        Some(&computed_style.font_family),
                                        cosmic_text::Weight::NORMAL,
                                        None,
                                        letter_spacing,
                                        word_spacing,
                                    );

                                    if sub_w <= safe_width {
                                        best_len = mid_adj;
                                        left = mid + 1;
                                        while left < text.len() && !text.is_char_boundary(left) {
                                            left += 1;
                                        }
                                    } else {
                                        right = mid.saturating_sub(1);
                                    }
                                }

                                let mut result_text = text[..best_len].to_string();
                                if use_ellipsis {
                                    result_text.push_str(ellipsis);
                                }
                                text = result_text;
                            } else if use_ellipsis {
                                text = ellipsis.to_string();
                            } else {
                                text = String::new(); // Clip everything
                            }
                        }
                    }
                    // -----------------------------------------------------

                    let mut canvas_data = None;
                    if element_tag == "canvas" {
                        let contexts = self.canvas_contexts.lock().unwrap();
                        if let Some(ctx2d) = contexts.get(&node_idx) {
                            canvas_data = Some(ctx2d.get_pixels().to_vec());
                        }
                    } else if element_tag == "svg" {
                        let svg_xml = dom.serialize_subtree_html(node_idx);
                        if let Some(pixels) =
                            crate::engine::svg::rasterize_svg_to_pixels(&svg_xml, w, h)
                        {
                            canvas_data = Some(pixels);
                        }
                    }

                    // ─── CLIPPING (OVERFLOW: HIDDEN / SCROLL / AUTO) ─────────
                    let mut clip_rect: Option<[f32; 4]> = None;
                    let mut current_ancestor = node.parent;
                    while let Some(pidx) = current_ancestor {
                        if let Some(pgeom) = geometry.get(&pidx) {
                            if pgeom.overflow_y
                                != crate::engine::style::css_values::CssOverflow::Visible
                                || pgeom.overflow_x
                                    != crate::engine::style::css_values::CssOverflow::Visible
                            {
                                let cr = [pgeom.x, pgeom.y, pgeom.width, pgeom.height];
                                clip_rect = Some(match clip_rect {
                                    Some(c) => {
                                        // Intersect rects
                                        let x1 = c[0].max(cr[0]);
                                        let y1 = c[1].max(cr[1]);
                                        let x2 = (c[0] + c[2]).min(cr[0] + cr[2]);
                                        let y2 = (c[1] + c[3]).min(cr[1] + cr[3]);
                                        let w = (x2 - x1).max(0.0);
                                        let h = (y2 - y1).max(0.0);
                                        [x1, y1, w, h]
                                    }
                                    None => cr,
                                });
                            }
                        }
                        current_ancestor = dom.get_node(pidx).and_then(|n| n.parent);
                    }
                    // ─── FIM CLIPPING ───────────────────────────────────────

                    let is_fixed = is_sticky_fixed
                        || matches!(
                            computed_style.position,
                            crate::engine::style::css_values::CssPosition::Fixed
                        );

                    let prim = DisplayItem {
                        x,
                        y,
                        width: w,
                        height: h,
                        background_color,
                        border_width: geom
                            .border_top
                            .max(geom.border_right)
                            .max(geom.border_bottom)
                            .max(geom.border_left),
                        border_color,
                        border_style: crate::engine::types::BorderStyle::Solid,
                        text_content: if text.is_empty() {
                            None
                        } else {
                            Some(std::sync::Arc::from(text.as_str()))
                        },
                        text_color,
                        text_overflow: computed_style.text_overflow.clone(),
                        font_size: computed_style.font_size,
                        letter_spacing: crate::engine::style::css_values::resolve_length(
                            &computed_style.letter_spacing,
                            computed_style.font_size,
                            16.0,
                            vw,
                            vh,
                        ),
                        word_spacing: crate::engine::style::css_values::resolve_length(
                            &computed_style.word_spacing,
                            computed_style.font_size,
                            16.0,
                            vw,
                            vh,
                        ),
                        image_url: None,
                        link_url: None,
                        node_idx,
                        element_type: crate::engine::types::ElementRenderType::from_str(
                            &element_tag,
                        ),
                        is_fixed,
                        opacity: computed_style.opacity,
                        border_radius: [
                            computed_style.border_radius_top_left,
                            computed_style.border_radius_top_right,
                            computed_style.border_radius_bottom_right,
                            computed_style.border_radius_bottom_left,
                        ],
                        transform_rotate: 0.0,
                        transform_scale: (1.0, 1.0),
                        transform_translate: (0.0, 0.0),
                        canvas_data,
                        input_value: if element_tag == "input"
                            || element_tag == "textarea"
                            || element_tag == "select"
                        {
                            if let crate::engine::dom::AceNodeType::Element(el) = &node.node_type {
                                std::sync::Arc::from(
                                    el.attributes
                                        .get("value")
                                        .map(|s| s.as_str())
                                        .unwrap_or_default(),
                                )
                            } else {
                                std::sync::Arc::from("")
                            }
                        } else {
                            std::sync::Arc::from("")
                        },
                        placeholder: if let crate::engine::dom::AceNodeType::Element(el) =
                            &node.node_type
                        {
                            std::sync::Arc::from(
                                el.attributes
                                    .get("placeholder")
                                    .map(|s| s.as_str())
                                    .unwrap_or_default(),
                            )
                        } else {
                            std::sync::Arc::from("")
                        },
                        input_type: if element_tag == "input" {
                            if let crate::engine::dom::AceNodeType::Element(el) = &node.node_type {
                                crate::engine::types::FormInputType::from_str(
                                    el.attributes
                                        .get("type")
                                        .map(|s| s.as_str())
                                        .unwrap_or("text"),
                                )
                            } else {
                                crate::engine::types::FormInputType::None
                            }
                        } else {
                            crate::engine::types::FormInputType::None
                        },
                        input_min: if element_tag == "input" {
                            if let crate::engine::dom::AceNodeType::Element(el) = &node.node_type {
                                std::sync::Arc::from(
                                    el.attributes
                                        .get("min")
                                        .map(|s| s.as_str())
                                        .unwrap_or_default(),
                                )
                            } else {
                                std::sync::Arc::from("")
                            }
                        } else {
                            std::sync::Arc::from("")
                        },
                        input_max: if element_tag == "input" {
                            if let crate::engine::dom::AceNodeType::Element(el) = &node.node_type {
                                std::sync::Arc::from(
                                    el.attributes
                                        .get("max")
                                        .map(|s| s.as_str())
                                        .unwrap_or_default(),
                                )
                            } else {
                                std::sync::Arc::from("")
                            }
                        } else {
                            std::sync::Arc::from("")
                        },
                        input_step: if element_tag == "input" {
                            if let crate::engine::dom::AceNodeType::Element(el) = &node.node_type {
                                std::sync::Arc::from(
                                    el.attributes
                                        .get("step")
                                        .map(|s| s.as_str())
                                        .unwrap_or_default(),
                                )
                            } else {
                                std::sync::Arc::from("")
                            }
                        } else {
                            std::sync::Arc::from("")
                        },
                        options: if element_tag == "select" {
                            let mut opts = Vec::new();
                            for &child_idx in &node.children {
                                if let Some(child) = dom.get_node(child_idx) {
                                    if let crate::engine::dom::AceNodeType::Element(child_el) =
                                        &child.node_type
                                    {
                                        if child_el.tag == "option" {
                                            let txt = child.get_text_content();
                                            if !txt.is_empty() {
                                                opts.push(txt);
                                            }
                                        }
                                    }
                                }
                            }
                            std::sync::Arc::from(opts.join("|").as_str())
                        } else {
                            std::sync::Arc::from("")
                        },
                        padding_top: geom.padding_top,
                        padding_right: geom.padding_right,
                        padding_bottom: geom.padding_bottom,
                        padding_left: geom.padding_left,
                        font_weight: computed_style.font_weight.clone(),
                        white_space: computed_style.white_space.clone(),
                        is_hovered: self.hovered_element == Some(node_idx),
                        is_focused: self.focused_element == Some(node_idx),
                        clip_rect,
                    };

                    if let Some(outline) = &computed_style.outline {
                        // outline-style: none => não renderiza
                        if outline.style != "none" {
                            let outline_color = Self::css_color_to_skia(&outline.color);
                            let total_gap = outline.width + outline.offset;
                            let outline_prim = DisplayItem {
                                x: x - total_gap,
                                y: y - total_gap,
                                width: w + total_gap * 2.0,
                                height: h + total_gap * 2.0,
                                background_color: None,
                                border_width: outline.width,
                                border_color: outline_color,
                                border_style: crate::engine::types::BorderStyle::from_str(
                                    &outline.style,
                                ),
                                text_content: None,
                                text_color: tiny_skia::Color::BLACK,
                                text_overflow:
                                    crate::engine::style::css_values::CssTextOverflow::Clip,
                                font_size: 0.0,
                                letter_spacing: 0.0,
                                word_spacing: 0.0,
                                image_url: None,
                                link_url: None,
                                node_idx,
                                element_type: crate::engine::types::ElementRenderType::Other,
                                is_fixed: false,
                                opacity: 1.0,
                                border_radius: [
                                    (computed_style.border_radius_top_left + total_gap).max(0.0),
                                    (computed_style.border_radius_top_right + total_gap).max(0.0),
                                    (computed_style.border_radius_bottom_right + total_gap)
                                        .max(0.0),
                                    (computed_style.border_radius_bottom_left + total_gap).max(0.0),
                                ],
                                transform_rotate: 0.0,
                                transform_scale: (1.0, 1.0),
                                transform_translate: (0.0, 0.0),
                                canvas_data: None,
                                input_value: std::sync::Arc::from(""),
                                placeholder: std::sync::Arc::from(""),
                                input_type: crate::engine::types::FormInputType::None,
                                input_min: std::sync::Arc::from(""),
                                input_max: std::sync::Arc::from(""),
                                input_step: std::sync::Arc::from(""),
                                options: std::sync::Arc::from(""),
                                padding_top: 0.0,
                                padding_right: 0.0,
                                padding_bottom: 0.0,
                                padding_left: 0.0,
                                font_weight:
                                    crate::engine::style::css_values::CssFontWeight::Normal,
                                white_space:
                                    crate::engine::style::css_values::CssWhiteSpace::Normal,
                                is_hovered: false,
                                is_focused: false,
                                clip_rect: None,
                            };
                            items.push(outline_prim);
                        }
                    }
                    // Backdrop para <dialog data-ace-modal> (modal)
                    if element_tag == "dialog" {
                        if let crate::engine::dom::AceNodeType::Element(el) = &node.node_type {
                            if el.attributes.contains_key("data-ace-modal") {
                                let backdrop = DisplayItem {
                                    x: 0.0,
                                    y: 0.0,
                                    width: vw,
                                    height: vh,
                                    background_color: Some(tiny_skia::Color::from_rgba8(
                                        0, 0, 0, 76,
                                    )),
                                    border_width: 0.0,
                                    border_color: None,
                                    border_style: crate::engine::types::BorderStyle::None,
                                    text_content: None,
                                    text_color: tiny_skia::Color::BLACK,
                                    text_overflow:
                                        crate::engine::style::css_values::CssTextOverflow::Clip,
                                    font_size: 0.0,
                                    letter_spacing: 0.0,
                                    word_spacing: 0.0,
                                    image_url: None,
                                    link_url: None,
                                    node_idx,
                                    element_type: crate::engine::types::ElementRenderType::Other,
                                    is_fixed: true,
                                    opacity: 1.0,
                                    border_radius: [0.0; 4],
                                    transform_rotate: 0.0,
                                    transform_scale: (1.0, 1.0),
                                    transform_translate: (0.0, 0.0),
                                    canvas_data: None,
                                    input_value: std::sync::Arc::from(""),
                                    placeholder: std::sync::Arc::from(""),
                                    input_type: crate::engine::types::FormInputType::None,
                                    input_min: std::sync::Arc::from(""),
                                    input_max: std::sync::Arc::from(""),
                                    input_step: std::sync::Arc::from(""),
                                    options: std::sync::Arc::from(""),
                                    padding_top: 0.0,
                                    padding_right: 0.0,
                                    padding_bottom: 0.0,
                                    padding_left: 0.0,
                                    font_weight:
                                        crate::engine::style::css_values::CssFontWeight::Normal,
                                    white_space:
                                        crate::engine::style::css_values::CssWhiteSpace::Normal,
                                    is_hovered: false,
                                    is_focused: false,
                                    clip_rect: None,
                                };
                                if backdrop.is_fixed {
                                    fixed_nodes.push(node_idx);
                                }
                                items.push(backdrop);
                            }
                        }
                    }
                    if prim.is_fixed {
                        fixed_nodes.push(node_idx);
                    }
                    items.push(prim);
                }
            }
        }
        crate::engine::layer_tree::LayerTree::build(items, &fixed_nodes)
    }

    fn is_technical_tag(&self, tag: &str) -> bool {
        matches!(
            tag,
            "style" | "script" | "head" | "meta" | "link" | "title" | "template"
        )
    }

    fn has_technical_ancestor(&self, dom: &crate::engine::dom::AceDOM, node_idx: usize) -> bool {
        let mut curr = dom.nodes.get(node_idx).and_then(|n| n.parent);
        while let Some(idx) = curr {
            if let Some(node) = dom.get_node(idx) {
                if let crate::engine::dom::AceNodeType::Element(el) = &node.node_type {
                    if self.is_technical_tag(&el.tag) {
                        return true;
                    }
                }
                curr = node.parent;
            } else {
                break;
            }
        }
        false
    }

    pub fn process_resource_responses(&mut self) -> bool {
        // Stub: processa respostas de recursos
        false
    }

    pub fn load_html(&mut self, html: &str) {
        use html5ever::tendril::TendrilSink;
        use kuchiki::parse_html;

        println!("[AceEngine] Parsing HTML...");
        let dom_tree = parse_html().one(html);
        let ace_dom = AceDOM::from_kuchiki(dom_tree);
        println!(
            "[AceEngine] DOM Tree created with {} nodes",
            ace_dom.nodes.len()
        );

        self.dom = Some(Arc::new(Mutex::new(ace_dom)));

        // Compilação do CSS da Página e injeção do Author CSS em self.stylesheet
        self.update_stylesheet();

        // Calcular estilos de todos os nós para habilitar o Display e Box Model
        self.recompute_dirty_styles();

        // Force layout computation immediately (this creates subframe slots for iframes)
        self.recompute_layout();

        // After layout is done and all DOM locks are released, initialize JS runtimes
        // for any iframe subframes that were just created. We do this OUTSIDE of all
        // DOM locks to avoid nested lock hangs (init_js_for_url runs init_stdlib which
        // uses tokio and may block).
        self.init_subframe_runtimes();
    }

    /// Initialize JS runtimes for all subframe iframes that don't have one yet.
    /// Must be called when NO DOM locks are held.
    fn init_subframe_runtimes(&mut self) {
        // Collect (node_idx, url, Arc<Mutex<AceEngine>>) for subframes that need a runtime
        let subframes_to_init: Vec<(usize, String, std::sync::Arc<std::sync::Mutex<AceEngine>>)> = {
            if let Some(ref dom_arc) = self.dom {
                let dom = dom_arc.lock().unwrap();
                if let Some(ref subframes_arc) = dom.subframes {
                    let subframes = subframes_arc.lock().unwrap();
                    subframes
                        .iter()
                        .filter(|(_, eng_arc)| {
                            let eng = eng_arc.lock().unwrap();
                            eng.js_runtime.is_none() && !eng.current_url.is_empty()
                        })
                        .map(|(idx, eng_arc)| {
                            let url = eng_arc.lock().unwrap().current_url.clone();
                            (*idx, url, eng_arc.clone())
                        })
                        .collect()
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        };
        // No DOM/subframes locks held from here on
        for (_, url, sub_engine_arc) in subframes_to_init {
            // Create a temporary snapshot of the engine for init_js_for_url (needs resource_manager etc.)
            let snap = sub_engine_arc.lock().unwrap().clone();
            // init_js_for_url does NOT require any external locks - it creates a fresh JsRuntime
            if let Some(rt) = crate::runtime::core::init::init_js_for_url(&url, &snap) {
                sub_engine_arc.lock().unwrap().js_runtime = Some(rt);
            }
        }
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

#[derive(Debug, Clone)]
pub struct FloatRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl FloatRect {
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }
    pub fn right(&self) -> f32 {
        self.x + self.width
    }
}

#[derive(Default, Debug, Clone)]
pub struct FloatContext {
    pub left_floats: Vec<FloatRect>,
    pub right_floats: Vec<FloatRect>,
}

impl FloatContext {
    pub fn get_left_offset(&self, y_min: f32, y_max: f32) -> f32 {
        let mut offset = 0.0_f32;
        for f in &self.left_floats {
            if y_min < f.bottom() && y_max > f.y {
                if f.right() > offset {
                    offset = f.right();
                }
            }
        }
        offset
    }

    pub fn get_right_offset(&self, y_min: f32, y_max: f32, container_width: f32) -> f32 {
        let mut limit = container_width;
        for f in &self.right_floats {
            if y_min < f.bottom() && y_max > f.y {
                if f.x < limit {
                    limit = f.x;
                }
            }
        }
        limit
    }

    pub fn get_clear_y(
        &self,
        clear_type: &crate::engine::style::css_values::CssClear,
        current_y: f32,
    ) -> f32 {
        use crate::engine::style::css_values::CssClear;
        let mut new_y = current_y;

        let check_left = matches!(clear_type, CssClear::Left | CssClear::Both);
        let check_right = matches!(clear_type, CssClear::Right | CssClear::Both);

        if check_left {
            for f in &self.left_floats {
                if f.bottom() > new_y {
                    new_y = f.bottom();
                }
            }
        }
        if check_right {
            for f in &self.right_floats {
                if f.bottom() > new_y {
                    new_y = f.bottom();
                }
            }
        }

        new_y
    }
}

#[derive(Clone, Debug)]
pub struct DisplayItem {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub background_color: Option<tiny_skia::Color>,
    pub border_width: f32,
    pub border_color: Option<tiny_skia::Color>,
    pub text_content: Option<std::sync::Arc<str>>,
    pub text_color: tiny_skia::Color,
    pub text_overflow: crate::engine::style::css_values::CssTextOverflow,
    pub font_size: f32,
    pub image_url: Option<std::sync::Arc<str>>,
    pub link_url: Option<std::sync::Arc<str>>,
    pub node_idx: usize,
    pub element_type: crate::engine::types::ElementRenderType,
    pub is_fixed: bool,
    pub opacity: f32,
    pub border_radius: [f32; 4],
    pub transform_rotate: f32,
    pub transform_scale: (f32, f32),
    pub transform_translate: (f32, f32),
    pub canvas_data: Option<Vec<u8>>,

    // Spacing
    pub letter_spacing: f32,
    pub word_spacing: f32,

    // Form Extensions
    pub input_value: std::sync::Arc<str>,
    pub placeholder: std::sync::Arc<str>,
    pub input_type: crate::engine::types::FormInputType,
    pub input_min: std::sync::Arc<str>,
    pub input_max: std::sync::Arc<str>,
    pub input_step: std::sync::Arc<str>,
    pub options: std::sync::Arc<str>,

    // Padding for box model rendering
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,

    // CSS text color mapped earlier
    // CSS font-weight ("normal", "bold", "100"-"900")
    pub font_weight: crate::engine::style::css_values::CssFontWeight,
    // CSS white-space ("normal", "nowrap", "pre", etc.)
    pub white_space: crate::engine::style::css_values::CssWhiteSpace,
    // Border style ("solid", "dashed", "dotted", "none", etc.)
    pub border_style: crate::engine::types::BorderStyle,
    pub is_hovered: bool,
    pub is_focused: bool,

    // Clipping viewport (for overflow: hidden/scroll/auto)
    pub clip_rect: Option<[f32; 4]>,
}

impl std::fmt::Debug for AceEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AceEngine")
            .field("current_url", &self.current_url)
            .field("styles_dirty", &self.styles_dirty)
            .finish()
    }
}
