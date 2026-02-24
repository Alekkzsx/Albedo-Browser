pub mod dom;
pub mod style;
pub mod graphics;
pub mod svg;
pub mod text;

use std::sync::{Arc, Mutex};
use crate::engine::dom::{AceDOM, AceNodeType};
use self::style::Stylesheet;
use self::style::css_values::{CssAlignItems, CssAlignContent, CssBoxSizing, CssFontWeight, CssFilter, TransformFunction};
use taffy::prelude::*;
use taffy::geometry::MinMax;

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
    pub overflow_x: String,
    pub overflow_y: String,
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
            overflow_x: "visible".to_string(),
            overflow_y: "visible".to_string(),
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
    pub styles_dirty: bool,
    pub animation_manager: Arc<Mutex<crate::engine::style::animation::AnimationManager>>,
    pub canvas_contexts: Arc<Mutex<std::collections::HashMap<usize, crate::engine::graphics::canvas2d::Canvas2D>>>,
    pub taffy: Arc<Mutex<taffy::Taffy>>,
    pub viewport_y: f32,
    /// Geometrias de elementos em subframes projetadas para o espaço global do frame pai.
    /// Chave: (iframe_node_idx * 1_000_000) + elem_node_idx_no_subframe
    /// Atualizado por collect_subframe_geometries() a cada layout.
    pub iframe_projected_geometry: Arc<Mutex<std::collections::HashMap<u64, ElementGeometry>>>,
}

impl AceEngine {
    pub fn new() -> Self {
        Self {
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
            styles_dirty: false,
            animation_manager: Arc::new(Mutex::new(crate::engine::style::animation::AnimationManager::new())),
            canvas_contexts: Arc::new(Mutex::new(std::collections::HashMap::new())),
            taffy: Arc::new(Mutex::new(taffy::Taffy::new())),
            viewport_y: 0.0,
            iframe_projected_geometry: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn load_url(&mut self, url: String) {
        println!("[AceEngine] Initiating load for URL: {}", url);
        self.current_url = url.clone();
        
        if let Some(rm) = &self.resource_manager {
            rm.fetch(url, crate::network::resources::ResourceType::Html, None);
        }
    }

    pub fn handle_resource_response(&mut self, res: crate::network::resources::ResourceResponse) -> bool {
        let mut needs_layout = false;
        
        if res.url == self.current_url {
             if let Ok(html) = String::from_utf8(res.data.clone()) {
                 println!("[AceEngine] Main document downloaded. Calling process_html...");
                 self.load_html(&html);
                 println!("[AceEngine] process_html returned successfully.");
                 needs_layout = true;
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
                let root_nodes = self.build_layout_tree(&mut dom, &mut taffy, &stylesheet, 0, width, height, &mut node_map, None);
                
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
                        self.extract_layout_recursively(&taffy, root_node, &node_map, &mut geometry, 0.0, 0.0);
                    } else {
                        println!("[AceEngine] Layout computation failed");
                    }
                }
            }
        }
    }

    fn extract_layout_recursively(&self, 
        taffy: &taffy::Taffy,
        node: taffy::prelude::Node,
        node_map: &std::collections::HashMap<taffy::prelude::Node, usize>,
        geometry: &mut std::collections::HashMap<usize, ElementGeometry>,
        parent_x: f32,
        parent_y: f32
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
    fn extract_box_model(&self, node_idx: usize) -> (f32, f32, f32, f32, f32, f32, f32, f32) {
        // TODO: Read from DOM computed style
        // For now, return zeros
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    }

    /// Calculate content dimensions based on children bounds.
    /// Returns: (content_width, content_height)
    fn calculate_content_dimensions(&self, node_idx: usize) -> (f32, f32) {
        // TODO: Iterate children and find max bounds
        // For now, return zeros
        (0.0, 0.0)
    }

    /// Get overflow style (overflow-x, overflow-y) from element.
    /// Returns: (overflow_x, overflow_y)
    fn get_overflow_style(&self, node_idx: usize) -> (String, String) {
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
            if x >= geom.x && x < (geom.x + geom.width) && y >= geom.y && y < (geom.y + geom.height) {
                // Caixas com z-index maior são renderizadas por último
                // Para simplificar, usamos o index como z-order (elementos adicionados depois têm z-index maior)
                let z_index = *node_idx as f32;
                match topmost {
                    None => topmost = Some((*node_idx, z_index)),
                    Some((_, current_z)) if z_index > current_z => topmost = Some((*node_idx, z_index)),
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
        self.styles_dirty = true;
    }

    pub fn recompute_dirty_styles(&mut self) {
        if !self.styles_dirty {
            return;
        }

        // Recompilar estilos para toda a árvore DOM
        if let Some(ref dom_arc) = self.dom {
            let mut dom = dom_arc.lock().unwrap();
            let stylesheet = self.stylesheet.lock().unwrap();
            
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64();
            let mut am = self.animation_manager.lock().unwrap();
            
            // Recompilar estilos com novos estados de hover/focus/active
            for idx in 0..dom.nodes.len() {
                let computed = stylesheet.calculate_style(
                    &dom,
                    idx,
                    None,
                    None,
                    self.hovered_element,
                    self.focused_element,
                    self.active_element,
                    Some(&am),
                    now,
                    800.0,
                    600.0,
                    "light"
                );
                
                // Start Keyframe Animations
                for anim_def in &computed.animations {
                    if let Some(keyframes) = stylesheet.keyframes.get(&anim_def.name) {
                        // Convert ComputedStyle TimingFunction to Animation TimingFunction
                            // For MVP, assuming they are compatible or mapping them.
                            // Actually, css_values::TimingFunction might differ from animation::TimingFunction?
                            // Let's check imports.
                            // In animation.rs: pub enum TimingFunction ...
                            // In css_values.rs: likely similar.
                            // We need to map or clone if they are same type.
                            // Assuming they are different types for now given module structure.
                            
                            // Let's map basic ones.
                            let timing = crate::engine::style::animation::TimingFunction::Ease; // Simplify for now or map properly
                            
                            am.start_keyframe_animation(
                                idx, 
                                anim_def.name.clone(), 
                                keyframes.clone(), 
                                anim_def.duration_ms as f64 / 1000.0, 
                                timing, 
                                now
                            );
                        }
                    }
                }
        }

        self.styles_dirty = false;
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
            self.styles_dirty = true;
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
        let mut style_dirty = false;

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
            let dom = dom_arc.lock().unwrap();
            // A implementação real do AceDOM pode ter flags para isso
            // mutated = dom.has_pending_mutations();
        }

        (mutated, style_dirty)
    }

    pub fn update_stylesheet(&mut self) {
        // Stub: atualiza stylesheet
    }

    pub fn recompute_layout(&mut self) {
        let start_time = std::time::Instant::now();
        if let Some(ref dom_arc) = self.dom {
            let mut dom = dom_arc.lock().unwrap();
            let mut taffy = self.taffy.lock().unwrap();
            let stylesheet = self.stylesheet.lock().unwrap();
            
            // 1. Build Taffy Tree
            taffy.clear();
            let mut node_map = std::collections::HashMap::new();
            
            // Clear geometry before rebuilding
            {
                let mut geometry = self.element_geometry.lock().unwrap();
                geometry.clear();
            }

            let build_start = std::time::Instant::now();
            let root_nodes = self.build_layout_tree(&mut dom, &mut taffy, &stylesheet, 0, 800.0, 600.0, &mut node_map, None);
            let build_duration = build_start.elapsed();
            
            if root_nodes.is_empty() { return; }
            let root_node = root_nodes[0]; 
            println!("[AceEngine] Taffy tree built, root nodes: {}", root_nodes.len());
            
            // 2. Compute Layout
            let compute_start = std::time::Instant::now();
            let size = taffy::geometry::Size {
                width: taffy::style::AvailableSpace::Definite(800.0),
                height: taffy::style::AvailableSpace::MaxContent,
            };
            let _ = taffy.compute_layout(root_node, size);
            let compute_duration = compute_start.elapsed();
            
            // 3. Update Element Bounds
            self.sync_taffy_bounds(&taffy, &dom, root_node, 0.0, 0.0, &node_map);
            
            let total_duration = start_time.elapsed();
            println!("[AceEngine] Layout Recomputed: Total={:?}, Build={:?}, Compute={:?}", total_duration, build_duration, compute_duration);
        }

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

                let visible_w = (clip_x2 - clip_x1).max(0.0);
                let visible_h = (clip_y2 - clip_y1).max(0.0);

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


    fn build_layout_tree(&self, 
        dom: &mut AceDOM, 
        taffy: &mut taffy::Taffy, 
        stylesheet: &Stylesheet, 
        node_idx: usize, 
        vw: f32, 
        vh: f32,
        node_map: &mut std::collections::HashMap<taffy::prelude::Node, usize>,
        parent_grid_ctx: Option<&GridContext>
    ) -> Vec<taffy::prelude::Node> {
        let (node_type, children_indices) = {
            let node = dom.get_node(node_idx).unwrap();
            (node.node_type.clone(), node.children.clone())
        };
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64();
        
        let style = {
            let am = self.animation_manager.lock().unwrap();
            stylesheet.calculate_style(
                dom, node_idx, None, None, 
                self.hovered_element, self.focused_element, self.active_element, 
                Some(&am), now, vw, vh, "light"
            )
        };

        // --- Element Geometry: Style Extraction ---
        {
            let mut geometry = self.element_geometry.lock().unwrap();
            let geom = geometry.entry(node_idx).or_insert(ElementGeometry::new());
            
            // Resolve Box Model
            let resolve = |l: &crate::engine::style::css_values::CssLength| -> f32 {
                 crate::engine::style::css_values::resolve_length(l, style.font_size, 16.0, vw, vh)
            };
            
            geom.padding_top = resolve(&style.padding_top);
            geom.padding_right = resolve(&style.padding_right);
            geom.padding_bottom = resolve(&style.padding_bottom);
            geom.padding_left = resolve(&style.padding_left);
            
            geom.border_top = resolve(&style.border_width_top);
            geom.border_right = resolve(&style.border_width_right);
            geom.border_bottom = resolve(&style.border_width_bottom);
            geom.border_left = resolve(&style.border_width_left);
            
            use crate::engine::style::css_values::CssOverflow;
            geom.overflow_x = match style.overflow {
                CssOverflow::Visible => "visible".to_string(),
                CssOverflow::Hidden => "hidden".to_string(),
                CssOverflow::Scroll => "scroll".to_string(),
                CssOverflow::Auto => "auto".to_string(),
            };
            geom.overflow_y = geom.overflow_x.clone(); 
        }
        // ------------------------------------------

        let mut taffy_style = self.convert_to_taffy_style(&style);
        
        // Resolve manual grid placements if in grid container
        if let Some(ctx) = parent_grid_ctx {
            self.apply_grid_context(&mut taffy_style, &style, ctx);
        }

        // Check if we should skip this node (flattening)
        // subgrid also triggers flattening in this implementation to inherit tracks
        let is_subgrid_cols = style.grid_template_columns.contains(&crate::engine::style::css_values::CssLength::Subgrid);
        let is_subgrid_rows = style.grid_template_rows.contains(&crate::engine::style::css_values::CssLength::Subgrid);
        let should_flatten = style.display == crate::engine::style::css_values::CssDisplay::Contents || is_subgrid_cols || is_subgrid_rows;

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
             let mut col_max_widths: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();

             fn scan_table_children(
                 dom: &AceDOM, 
                 node_idx: usize, 
                 row_count: &mut usize, 
                 col_count: &mut usize,
                 cell_list: &mut Vec<(usize, usize, usize)>,
                 col_max_widths: &mut std::collections::HashMap<usize, f32>,
                 font_size_cache: f32
             ) {
                 if let Some(node) = dom.get_node(node_idx) {
                     for &child_idx in &node.children {
                         if let Some(child_node) = dom.get_node(child_idx) {
                             if let crate::engine::dom::AceNodeType::Element(el) = &child_node.node_type {
                                 let tag = el.tag.as_str();
                                 if tag == "tr" {
                                     let mut current_cols = 0;
                                     for &cell_idx in &child_node.children {
                                         if let Some(cell_node) = dom.get_node(cell_idx) {
                                            if let crate::engine::dom::AceNodeType::Element(cell_el) = &cell_node.node_type {
                                                if cell_el.tag == "td" || cell_el.tag == "th" {
                                                    cell_list.push((*row_count, current_cols, cell_idx));
                                                    
                                                    // TWO-PASS: Buscar nó de texto filho pra saber a largura bruta em string
                                                    let mut text_len = 0.0;
                                                    for &inner_idx in &cell_node.children {
                                                        if let Some(inner_node) = dom.get_node(inner_idx) {
                                                            if let crate::engine::dom::AceNodeType::Text(text_str) = &inner_node.node_type {
                                                                 // (Rough character estimation * font_size / 2.0 = text width px)
                                                                 text_len += text_str.len() as f32 * (font_size_cache * 0.55);
                                                            }
                                                        }
                                                    }
                                                    // Add 20px of default padding to the text
                                                    text_len += 20.0;
                                                    
                                                    let max_w = col_max_widths.entry(current_cols).or_insert(0.0);
                                                    if text_len > *max_w { *max_w = text_len; }
                                                    
                                                    current_cols += 1;
                                                }
                                            }
                                         }
                                     }
                                     if current_cols > *col_count { *col_count = current_cols; }
                                     *row_count += 1;
                                 } else if tag == "thead" || tag == "tbody" || tag == "tfoot" {
                                     scan_table_children(dom, child_idx, row_count, col_count, cell_list, col_max_widths, font_size_cache);
                                 }
                             }
                         }
                     }
                 }
             }

             scan_table_children(dom, node_idx, &mut row_count, &mut col_count, &mut cell_list, &mut col_max_widths, style.font_size);

             // 2. Set Grid Template with Pre-calculated PX
             if col_count > 0 {
                  let mut tracks = Vec::new();
                  for i in 0..col_count {
                      let computed_px = *col_max_widths.get(&i).unwrap_or(&100.0);
                      tracks.push(taffy::prelude::TrackSizingFunction::Single(
                           taffy::geometry::MinMax {
                               min: taffy::prelude::MinTrackSizingFunction::Fixed(taffy::prelude::LengthPercentage::Points(computed_px)),
                               max: taffy::prelude::MaxTrackSizingFunction::Fixed(taffy::prelude::LengthPercentage::Points(computed_px)),
                           }
                      ));
                  }
                  taffy_style.grid_template_columns = tracks;
             }
             
             // 3. Create Children (Cells flattened)
             let mut children = Vec::new();
             for (r, c, cell_idx) in cell_list {
                let cell_nodes = self.build_layout_tree(dom, taffy, stylesheet, cell_idx, vw, vh, node_map, None);
                if let Some(&cell_taffy_node) = cell_nodes.first() {
                    let mut cell_style = taffy.style(cell_taffy_node).unwrap().clone();
                    // 1-based index for grid placement
                    cell_style.grid_row.start = taffy::prelude::GridPlacement::Line((r as i16 + 1).into());
                    cell_style.grid_column.start = taffy::prelude::GridPlacement::Line((c as i16 + 1).into());
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
            
            let resolve_line_local = |l: &crate::engine::style::css_values::CssLength, names: &std::collections::HashMap<String, Vec<i16>>| -> Option<i16> {
                use crate::engine::style::css_values::CssLength;
                match l {
                    CssLength::Number(v) => Some(*v as i16),
                    CssLength::Name(n) => names.get(n).and_then(|v| v.first().copied()),
                    CssLength::Span(v) => Some(*v as i16),
                    _ => None,
                }
            };

            // Calculate offsets based on subgrid's own placement in parent
            if let Some(start) = resolve_line_local(&style.grid_column_start, &parent_ctx.column_names) {
                ctx.col_offset += start - 1;
            } else if let crate::engine::style::css_values::CssLength::Name(name) = &style.grid_column_start {
                if let Some(area) = parent_ctx.areas.get(name) {
                    ctx.col_offset += (area.2 as i16) - 1;
                }
            }

            if let Some(start) = resolve_line_local(&style.grid_row_start, &parent_ctx.row_names) {
                ctx.row_offset += start - 1;
            } else if let crate::engine::style::css_values::CssLength::Name(name) = &style.grid_row_start {
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
                        dom.subframes = Some(std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())));
                    }
                    
                    let subframes_arc = dom.subframes.as_ref().unwrap().clone();
                    let mut subframes = subframes_arc.lock().unwrap();
                    
                    if !subframes.contains_key(&node_idx) {
                        let sub_engine = std::sync::Arc::new(std::sync::Mutex::new(AceEngine::new()));
                        // Marcar qual nó <iframe> este subframe representa no pai
                        {
                            let mut sub_eng = sub_engine.lock().unwrap();
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
                       if let Ok(w) = w_attr.parse::<f32>() { svg_width = crate::engine::style::css_values::CssLength::Px(w); }
                   }
                }
                if svg_height == crate::engine::style::css_values::CssLength::Auto {
                    if let Some(h_attr) = el.attributes.get("height") {
                        if let Ok(h) = h_attr.parse::<f32>() { svg_height = crate::engine::style::css_values::CssLength::Px(h); }
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
            // Text nodes need an intrinsic size estimate to be visible
            let font_size = style.font_size;
            let char_width = font_size * 0.5; // Rough estimate
            let text_width = text.len() as f32 * char_width;
            
            // Limit width to viewport if it's too long, causing wrap
            let width = if text_width > vw { vw } else { text_width };
            let height = font_size * (text_width / width).ceil();
            
            taffy_style.size.width = taffy::prelude::Dimension::Points(width);
            taffy_style.size.height = taffy::prelude::Dimension::Points(height);
            
            let taffy_node = taffy.new_leaf(taffy_style).unwrap();
            node_map.insert(taffy_node, node_idx);
            return vec![taffy_node];
        }
        
        let mut children = Vec::new();

        if style.display == crate::engine::style::css_values::CssDisplay::Table {
             // Already handled in build_layout_tree main block for tables
        } else if style.display == crate::engine::style::css_values::CssDisplay::TableRow {
            // Table rows are flattened, their children (cells) become direct children of the table grid
            for child_idx in children_indices {
                children.extend(self.build_layout_tree(dom, taffy, stylesheet, child_idx, vw, vh, node_map, grid_ctx.as_ref()));
            }
        } else if style.display == crate::engine::style::css_values::CssDisplay::TableHeader {
            // Table headers are also flattened, their children (cells) become direct children of the table grid
            for child_idx in children_indices {
                children.extend(self.build_layout_tree(dom, taffy, stylesheet, child_idx, vw, vh, node_map, grid_ctx.as_ref()));
            }
        } else {
             for child_idx in children_indices {
                children.extend(self.build_layout_tree(dom, taffy, stylesheet, child_idx, vw, vh, node_map, grid_ctx.as_ref()));
            }
        }

        if should_flatten {
            // If flattening, we don't create a Taffy node for THIS element.
            // We just return its children to be added to the grandparent.
            return children;
        }

        let taffy_node = taffy.new_with_children(taffy_style, &children).unwrap();
        node_map.insert(taffy_node, node_idx);
        vec![taffy_node]
    }

    fn apply_grid_context(&self, t_style: &mut taffy::prelude::Style, style: &crate::engine::style::css_values::ComputedStyle, ctx: &GridContext) {
        use crate::engine::style::css_values::CssLength;
        
        let resolve_line = |l: &CssLength, names: &std::collections::HashMap<String, Vec<i16>>| -> Option<i16> {
            match l {
                CssLength::Number(v) => Some(*v as i16),
                CssLength::Name(n) => names.get(n).and_then(|v| v.first().copied()),
                _ => None,
            }
        };

        if let Some(start) = resolve_line(&style.grid_column_start, &ctx.column_names) {
            t_style.grid_column.start = taffy::prelude::GridPlacement::Line((start + ctx.col_offset).into());
        }
        if let Some(end) = resolve_line(&style.grid_column_end, &ctx.column_names) {
            t_style.grid_column.end = taffy::prelude::GridPlacement::Line((end + ctx.col_offset).into());
        }
        if let Some(start) = resolve_line(&style.grid_row_start, &ctx.row_names) {
            t_style.grid_row.start = taffy::prelude::GridPlacement::Line((start + ctx.row_offset).into());
        }
        if let Some(end) = resolve_line(&style.grid_row_end, &ctx.row_names) {
            t_style.grid_row.end = taffy::prelude::GridPlacement::Line((end + ctx.row_offset).into());
        }

        // Apply grid-area name if it matches an area
        if let CssLength::Name(name) = &style.grid_row_start {
            if let Some(area) = ctx.areas.get(name) {
                t_style.grid_row.start = taffy::prelude::GridPlacement::Line(((area.0 as i16) + ctx.row_offset).into());
                t_style.grid_row.end = taffy::prelude::GridPlacement::Line(((area.1 as i16) + ctx.row_offset).into());
                t_style.grid_column.start = taffy::prelude::GridPlacement::Line(((area.2 as i16) + ctx.col_offset).into());
                t_style.grid_column.end = taffy::prelude::GridPlacement::Line(((area.3 as i16) + ctx.col_offset).into());
            }
        }
    }

    fn sync_taffy_bounds(&self, 
        taffy: &taffy::Taffy, 
        dom: &AceDOM, 
        taffy_node: taffy::prelude::Node, 
        offset_x: f32, 
        offset_y: f32,
        node_map: &std::collections::HashMap<taffy::prelude::Node, usize>
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
                        if right > content_w { content_w = right; }
                        if bottom > content_h { content_h = bottom; }
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

            for &taffy_child in &taffy_children {
                self.sync_taffy_bounds(taffy, dom, taffy_child, abs_x, abs_y, node_map);
            }
        }

    fn convert_to_taffy_style(&self, style: &crate::engine::style::css_values::ComputedStyle) -> taffy::prelude::Style {
        let mut t_style = taffy::prelude::Style::default();
        
        t_style.display = match style.display {
            crate::engine::style::css_values::CssDisplay::Grid | crate::engine::style::css_values::CssDisplay::Table => taffy::prelude::Display::Grid,
            crate::engine::style::css_values::CssDisplay::Flex => taffy::prelude::Display::Flex,
            crate::engine::style::css_values::CssDisplay::None => taffy::prelude::Display::None,
            crate::engine::style::css_values::CssDisplay::Contents | crate::engine::style::css_values::CssDisplay::TableRow | crate::engine::style::css_values::CssDisplay::TableHeader => taffy::prelude::Display::None, // Will be flattened/handled manually
            _ => taffy::prelude::Display::Flex,
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
            bottom: self.to_taffy_length_percentage(&style.padding_bottom).into(),
        };

        t_style.gap = taffy::prelude::Size {
            width: self.to_taffy_length_percentage(&style.grid_column_gap),
            height: self.to_taffy_length_percentage(&style.grid_row_gap),
        };

        // Filter out LineNames before passing to Taffy
        t_style.grid_template_columns = style.grid_template_columns.iter()
            .filter(|l| !matches!(l, crate::engine::style::css_values::CssLength::LineNames(_)))
            .map(|l| self.to_taffy_track_size(l))
            .collect();
            
        t_style.grid_template_rows = style.grid_template_rows.iter()
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

    fn resolve_placement(&self, placement: &crate::engine::style::css_values::CssLength) -> taffy::prelude::GridPlacement {
        use crate::engine::style::css_values::CssLength;
        match placement {
            CssLength::Number(v) => taffy::prelude::GridPlacement::Line((*v as i16).into()),
            CssLength::Span(v) => taffy::prelude::GridPlacement::Span(*v),
            CssLength::Name(n) => {
                // This is a placeholder; real resolution happens in build_layout_tree
                // if we have parent context.
                taffy::prelude::GridPlacement::Auto
            },
            _ => taffy::prelude::GridPlacement::Auto,
        }
    }

    fn resolve_grid_areas(&self, areas: &[String]) -> std::collections::HashMap<String, (usize, usize, usize, usize)> {
        let mut map = std::collections::HashMap::new();
        for (row_idx, row_str) in areas.iter().enumerate() {
            let cols: Vec<&str> = row_str.split_whitespace().collect();
            for (col_idx, area_name) in cols.iter().enumerate() {
                if *area_name == "." { continue; }
                let entry = map.entry(area_name.to_string()).or_insert((row_idx, row_idx + 1, col_idx, col_idx + 1));
                entry.0 = entry.0.min(row_idx);
                entry.1 = entry.1.max(row_idx + 1);
                entry.2 = entry.2.min(col_idx);
                entry.3 = entry.3.max(col_idx + 1);
            }
        }
        map
    }

    fn resolve_grid_names(&self, tracks: &[crate::engine::style::css_values::CssLength]) -> std::collections::HashMap<String, Vec<i16>> {
        let mut map = std::collections::HashMap::new();
        let mut line_idx = 0;
        for track in tracks {
            match track {
                crate::engine::style::css_values::CssLength::LineNames(names) => {
                    for name in names {
                        map.entry(name.clone()).or_insert(Vec::new()).push(line_idx);
                    }
                }
                _ => { line_idx += 1; }
            }
        }
        map
    }

    fn to_taffy_dimension(&self, len: &crate::engine::style::css_values::CssLength) -> taffy::prelude::Dimension {
        use crate::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::Dimension::Points(*v),
            CssLength::Percent(v) => taffy::prelude::Dimension::Percent(*v / 100.0),
            CssLength::Auto => taffy::prelude::Dimension::Auto,
            _ => taffy::prelude::Dimension::Auto,
        }
    }

    fn to_taffy_length_percentage(&self, len: &crate::engine::style::css_values::CssLength) -> taffy::prelude::LengthPercentage {
        use crate::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::LengthPercentage::Points(*v),
            CssLength::Percent(v) => taffy::prelude::LengthPercentage::Percent(*v / 100.0),
            _ => taffy::prelude::LengthPercentage::Points(0.0),
        }
    }

    fn to_taffy_track_size(&self, len: &crate::engine::style::css_values::CssLength) -> taffy::prelude::TrackSizingFunction {
        use crate::engine::style::css_values::CssLength;
        match len {
            CssLength::Fr(v) => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: taffy::prelude::MinTrackSizingFunction::Auto,
                max: taffy::prelude::MaxTrackSizingFunction::Fraction(*v),
            }),
            CssLength::Px(v) => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: taffy::prelude::MinTrackSizingFunction::Fixed(taffy::prelude::LengthPercentage::Points(*v)),
                max: taffy::prelude::MaxTrackSizingFunction::Fixed(taffy::prelude::LengthPercentage::Points(*v)),
            }),
            CssLength::Percent(v) => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: taffy::prelude::MinTrackSizingFunction::Fixed(taffy::prelude::LengthPercentage::Percent(*v / 100.0)),
                max: taffy::prelude::MaxTrackSizingFunction::Fixed(taffy::prelude::LengthPercentage::Percent(*v / 100.0)),
            }),
            CssLength::MinMax(min, max) => {
                taffy::prelude::TrackSizingFunction::Single(MinMax {
                    min: self.to_taffy_min_track(min),
                    max: self.to_taffy_max_track(max),
                })
            },
            CssLength::Repeat(count, sub) => {
                let repetition = match count.as_str() {
                    "auto-fill" => taffy::prelude::GridTrackRepetition::AutoFill,
                    "auto-fit" => taffy::prelude::GridTrackRepetition::AutoFit,
                    _ => {
                        let c = count.parse::<u16>().unwrap_or(1);
                        taffy::prelude::GridTrackRepetition::Count(c)
                    }
                };
                let tracks = sub.iter().map(|l| {
                    match self.to_taffy_track_size(l) {
                        taffy::prelude::TrackSizingFunction::Single(mm) => mm,
                        _ => MinMax { 
                            min: taffy::prelude::MinTrackSizingFunction::Auto, 
                            max: taffy::prelude::MaxTrackSizingFunction::Auto 
                        },
                    }
                }).collect();
                taffy::prelude::TrackSizingFunction::Repeat(repetition, tracks)
            },
            _ => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: taffy::prelude::MinTrackSizingFunction::Auto,
                max: taffy::prelude::MaxTrackSizingFunction::Auto,
            }),
        }
    }

    fn to_taffy_min_track(&self, len: &crate::engine::style::css_values::CssLength) -> taffy::prelude::MinTrackSizingFunction {
        use crate::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::MinTrackSizingFunction::Fixed(taffy::prelude::LengthPercentage::Points(*v)),
            CssLength::Percent(v) => taffy::prelude::MinTrackSizingFunction::Fixed(taffy::prelude::LengthPercentage::Percent(*v / 100.0)),
            CssLength::MinContent => taffy::prelude::MinTrackSizingFunction::MinContent,
            CssLength::MaxContent => taffy::prelude::MinTrackSizingFunction::MaxContent,
            _ => taffy::prelude::MinTrackSizingFunction::Auto,
        }
    }

    fn to_taffy_max_track(&self, len: &crate::engine::style::css_values::CssLength) -> taffy::prelude::MaxTrackSizingFunction {
        use crate::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::MaxTrackSizingFunction::Fixed(taffy::prelude::LengthPercentage::Points(*v)),
            CssLength::Percent(v) => taffy::prelude::MaxTrackSizingFunction::Fixed(taffy::prelude::LengthPercentage::Percent(*v / 100.0)),
            CssLength::Fr(v) => taffy::prelude::MaxTrackSizingFunction::Fraction(*v),
            CssLength::MinContent => taffy::prelude::MaxTrackSizingFunction::MinContent,
            CssLength::MaxContent => taffy::prelude::MaxTrackSizingFunction::MaxContent,
            _ => taffy::prelude::MaxTrackSizingFunction::Auto,
        }
    }

    pub fn render_visual(&self) -> Vec<VisualPrimitive> {
        // Renderização básica: converter elementos DOM em primitivas visuais
        // Cada elemento DOM vira uma VisualPrimitive com sua posição, tamanho e estilos
        let mut primitives = Vec::new();
        
        if let Some(ref dom_arc) = self.dom {
            let dom = dom_arc.lock().unwrap();
            let stylesheet = self.stylesheet.lock().unwrap();
            let geometry = self.element_geometry.lock().unwrap();
            
            // Iterar sobre todos os elementos e gerar primitivas
            for (node_idx, node) in dom.nodes.iter().enumerate() {
                let (is_element, element_tag) = match &node.node_type {
                    crate::engine::dom::AceNodeType::Element(el) => (true, el.tag.clone()),
                    crate::engine::dom::AceNodeType::Text(_) => (false, "#text".to_string()),
                    _ => continue, // Skip others
                };
                    // Calcular estilos com estados atuais (hover, focus, active)
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64();
                    let am = self.animation_manager.lock().unwrap();

                    let computed_style = stylesheet.calculate_style(
                        &dom,
                        node_idx,
                        None,
                        None,
                        self.hovered_element,
                        self.focused_element,
                        self.active_element,
                        Some(&am),
                        now,
                        800.0,
                        600.0,
                        "light"
                    );
                    
                    // Pular elementos com display: none
                    if matches!(computed_style.display, crate::engine::style::css_values::CssDisplay::None) {
                        continue;
                    }
                    
                    // Obter bounding box se disponível
                    if let Some(geom) = geometry.get(&node_idx) {
                        let x = geom.x;
                        let y = geom.y;
                        let w = geom.width;
                        let h = geom.height;
                        // Converter CssColor para hex string
                        let color_str = match &computed_style.background_color {
                            crate::engine::style::css_values::CssColor::Named(name) => {
                                if name.starts_with("#") {
                                    name.clone()
                                } else {
                                    // Conversão básica de nomes de cores
                                    match name.to_lowercase().as_str() {
                                        "white" => "#ffffff".to_string(),
                                        "black" => "#000000".to_string(),
                                        "red" => "#ff0000".to_string(),
                                        "green" => "#008000".to_string(),
                                        "blue" => "#0000ff".to_string(),
                                        "yellow" => "#ffff00".to_string(),
                                        "gray" | "grey" => "#808080".to_string(),
                                        "transparent" => "transparent".to_string(),
                                        _ => "transparent".to_string(),
                                    }
                                }
                            },
                            crate::engine::style::css_values::CssColor::Rgba(r, g, b, _a) => format!("#{:02x}{:02x}{:02x}", r, g, b),
                            crate::engine::style::css_values::CssColor::Transparent => "transparent".to_string(),
                            _ => "transparent".to_string(),
                        };
                        
                        // Extrair texto se disponível
                        let text = match &node.node_type {
                            crate::engine::dom::AceNodeType::Text(t) => t.clone(),
                            _ => String::new(),
                        };
                        
                        // Extrair dados de canvas se for um elemento <canvas>
                        let mut canvas_data = None;
                        if element_tag == "canvas" {
                            let contexts = self.canvas_contexts.lock().unwrap();
                            if let Some(ctx2d) = contexts.get(&node_idx) {
                                canvas_data = Some(ctx2d.get_pixels().to_vec());
                            }
                        } else if element_tag == "svg" {
                             // SVG Support: Rasterize the subtree
                             let svg_xml = dom.serialize_subtree_html(node_idx);
                             if let Some(pixels) = crate::engine::svg::rasterize_svg_to_pixels(&svg_xml, w, h) {
                                 canvas_data = Some(pixels);
                             }
                        }

                            // Criar primitiva visual
                            let prim = VisualPrimitive {
                                x: x,
                                y: y,
                                width: w,
                                height: h,
                                color: color_str,
                                text,
                                font_size: computed_style.font_size,
                                image_url: None,
                                link_url: None,
                                node_idx,
                                element_type: element_tag.clone(),
                                is_fixed: matches!(computed_style.position, crate::engine::style::css_values::CssPosition::Fixed),
                                opacity: computed_style.opacity,
                                border_radius: [
                                    computed_style.border_radius_top_left,
                                    computed_style.border_radius_top_right,
                                    computed_style.border_radius_bottom_right,
                                    computed_style.border_radius_bottom_left,
                                ],
                                transform_rotate: computed_style.transform.iter().find_map(|t| if let crate::engine::style::css_values::TransformFunction::Rotate(a) = t { Some(*a) } else { None }).unwrap_or(0.0),
                                transform_scale: computed_style.transform.iter().find_map(|t| if let crate::engine::style::css_values::TransformFunction::Scale(sx, sy) = t { Some((*sx, *sy)) } else { None }).unwrap_or((1.0, 1.0)),
                                transform_translate: computed_style.transform.iter().find_map(|t| if let crate::engine::style::css_values::TransformFunction::Translate(tx, ty) = t { 
                                    // Simplified: only support Px for now in bridge sync
                                    let tx_val = if let crate::engine::style::css_values::CssLength::Px(v) = tx { *v } else { 0.0 };
                                    let ty_val = if let crate::engine::style::css_values::CssLength::Px(v) = ty { *v } else { 0.0 };
                                    Some((tx_val, ty_val))
                                } else { None }).unwrap_or((0.0, 0.0)),
                                canvas_data,
                                
                                
                                // Form Data Extraction
                                input_value: if element_tag == "input" || element_tag == "textarea" || element_tag == "select" {
                                    if let crate::engine::dom::AceNodeType::Element(el) = &node.node_type {
                                        el.attributes.get("value").cloned().unwrap_or_default()
                                    } else { String::new() }
                                } else { String::new() },
                                
                                placeholder: if let crate::engine::dom::AceNodeType::Element(el) = &node.node_type {
                                    el.attributes.get("placeholder").cloned().unwrap_or_default()
                                } else { String::new() },
                                
                                input_type: if element_tag == "input" {
                                    if let crate::engine::dom::AceNodeType::Element(el) = &node.node_type {
                                        el.attributes.get("type").cloned().unwrap_or("text".to_string())
                                    } else { String::new() }
                                } else { String::new() },
                            
                            options: if element_tag == "select" {
                                // Extract options from children
                                let mut opts = Vec::new();
                                for &child_idx in &node.children {
                                    if let Some(child) = dom.get_node(child_idx) {
                                        if let crate::engine::dom::AceNodeType::Element(child_el) = &child.node_type {
                                            if child_el.tag == "option" {
                                                let txt = child.get_text_content();
                                                if !txt.is_empty() {
                                                     opts.push(txt);
                                                }
                                            }
                                        }
                                    }
                                }
                                opts.join("|")
                            } else { String::new() },
                            
                            padding_top: 0.0,
                            padding_right: 0.0,
                            padding_bottom: 0.0,
                            padding_left: 0.0,
                        };
                        
                        primitives.push(prim);
                        
                        // Se elemento tem outline (focus visual feedback), adicionar primitiva de outline
                        if let Some(outline) = &computed_style.outline {
                            let outline_color = match &outline.color {
                                crate::engine::style::css_values::CssColor::Named(name) => {
                                    if name.starts_with("#") {
                                        name.clone()
                                    } else {
                                        // Conversão básica de nomes de cores
                                        match name.to_lowercase().as_str() {
                                            "blue" => "#0066ff".to_string(),
                                            "red" => "#ff0000".to_string(),
                                            "green" => "#00aa00".to_string(),
                                            _ => "#0066ff".to_string(),
                                        }
                                    }
                                },
                                crate::engine::style::css_values::CssColor::Rgba(r, g, b, _a) => format!("#{:02x}{:02x}{:02x}", r, g, b),
                                _ => "#0066ff".to_string(),
                            };
                            
                            // Para simplificar, o outline é renderizado como um retângulo de borda
                            // No futuro, pode ser melhorado com shader de borda
                            let outline_prim = VisualPrimitive {
                                x: x - outline.offset,
                                y: y - outline.offset,
                                width: w + outline.offset * 2.0,
                                height: h + outline.offset * 2.0,
                                color: outline_color,
                                text: String::new(),
                                font_size: 0.0,
                                image_url: None,
                                link_url: None,
                                node_idx,
                                element_type: format!("outline-{}", element_tag),
                                is_fixed: false,
                                opacity: 1.0,
                                border_radius: [0.0; 4],
                                transform_rotate: 0.0,
                                transform_scale: (1.0, 1.0),
                                transform_translate: (0.0, 0.0),
                                canvas_data: None,
                                
                                input_value: String::new(),
                                placeholder: String::new(),
                                input_type: String::new(),
                                options: String::new(),
                                
                                padding_top: 0.0,
                                padding_right: 0.0,
                                padding_bottom: 0.0,
                                padding_left: 0.0,
                            };
                            
                            // Adicionar outline antes do elemento para que apareça atrás
                            // (será renderizado depois, assim aparecerá na frente)
                            primitives.insert(primitives.len() - 1, outline_prim);
                        }
                    }
                }
            }
        
        primitives
    }

    pub fn process_resource_responses(&mut self) -> bool {
        // Stub: processa respostas de recursos
        false
    }

    pub fn load_html(&mut self, html: &str) {
        use html5ever::parse_document;
        use html5ever::tendril::TendrilSink;
        use kuchiki::parse_html;
        
        println!("[AceEngine] Parsing HTML...");
        let dom_tree = parse_html().one(html);
        let ace_dom = AceDOM::from_kuchiki(dom_tree);
        println!("[AceEngine] DOM Tree created with {} nodes", ace_dom.nodes.len());
        
        self.dom = Some(Arc::new(Mutex::new(ace_dom)));
        
        // Force layout computation immediately (this creates subframe slots for iframes)
        self.recompute_layout();

        // After layout is done and all DOM locks are released, initialize JS runtimes
        // for any iframe subframes that were just created. We do this OUTSIDE of all
        // DOM locks to avoid nested lock hangs (init_js_for_url runs init_stdlib which
        // uses tokio channels).
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
                    subframes.iter()
                        .filter(|(_, eng_arc)| {
                            let eng = eng_arc.lock().unwrap();
                            eng.js_runtime.is_none() && !eng.current_url.is_empty()
                        })
                        .map(|(idx, eng_arc)| {
                            let url = eng_arc.lock().unwrap().current_url.clone();
                            (*idx, url, eng_arc.clone())
                        })
                        .collect()
                } else { vec![] }
            } else { vec![] }
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
            styles_dirty: self.styles_dirty,
            animation_manager: self.animation_manager.clone(),
            canvas_contexts: self.canvas_contexts.clone(),
            taffy: self.taffy.clone(),
            viewport_y: self.viewport_y,
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
    pub opacity: f32,
    pub border_radius: [f32; 4],
    pub transform_rotate: f32,
    pub transform_scale: (f32, f32),
    pub transform_translate: (f32, f32),
    pub canvas_data: Option<Vec<u8>>,
    
    // Form Extensions
    pub input_value: String,
    pub placeholder: String,
    pub input_type: String,
    pub options: String,
    
    // Padding for box model rendering
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,
}

impl std::fmt::Debug for AceEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AceEngine")
            .field("current_url", &self.current_url)
            .field("styles_dirty", &self.styles_dirty)
            .finish()
    }
}
