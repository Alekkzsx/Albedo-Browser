
use crate::ace::engine::dom::AceDOM;
use crate::ace::engine::layout::ElementGeometry;
use crate::ace::engine::core::AceEngine;


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
        clear_type: &crate::ace::engine::style::css_values::CssClear,
        current_y: f32,
    ) -> f32 {
        use crate::ace::engine::style::css_values::CssClear;
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

impl AceEngine {
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
                node.dirty = crate::ace::engine::dom::NodeDirtyFlags::NONE;
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
    pub(crate) fn extract_layout_recursively(
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
    pub(crate) fn sync_taffy_bounds(
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
    pub(crate) fn apply_floats_recursive(
        &self,
        node_idx: usize,
        dom: &crate::ace::engine::dom::AceDOM,
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

            if my_clear != crate::ace::engine::style::css_values::CssClear::None {
                let new_y = float_ctx.get_clear_y(&my_clear, geom.y);
                if new_y > geom.y {
                    let dy = new_y - geom.y;
                    geom.y = new_y;
                    current_offset_y += dy;
                }
            }

            if my_float == crate::ace::engine::style::css_values::CssFloat::Left {
                let left_edge = float_ctx.get_left_offset(geom.y, geom.y + geom.height);
                geom.x = geom.x.max(left_edge);

                float_ctx.left_floats.push(FloatRect {
                    x: geom.x,
                    y: geom.y,
                    width: geom.width,
                    height: geom.height,
                });
            } else if my_float == crate::ace::engine::style::css_values::CssFloat::Right {
                float_ctx.right_floats.push(FloatRect {
                    x: geom.x,
                    y: geom.y,
                    width: geom.width,
                    height: geom.height,
                });
            } else if my_float == crate::ace::engine::style::css_values::CssFloat::None
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
    /// Extract box model (borders and padding) from element.
    /// Returns: (border_top, border_right, border_bottom, border_left, padding_top, padding_right, padding_bottom, padding_left)
    pub(crate) fn _extract_box_model(&self, _node_idx: usize) -> (f32, f32, f32, f32, f32, f32, f32, f32) {
        // TODO: Read from DOM computed style
        // For now, return zeros
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    }
    /// Calculate content dimensions based on children bounds.
    /// Returns: (content_width, content_height)
    pub(crate) fn _calculate_content_dimensions(&self, _node_idx: usize) -> (f32, f32) {
        // TODO: Iterate children and find max bounds
        // For now, return zeros
        (0.0, 0.0)
    }
    /// Get overflow style (overflow-x, overflow-y) from element.
    /// Returns: (String, String)
    pub(crate) fn _get_overflow_style(&self, _node_idx: usize) -> (String, String) {
        // TODO: Read from DOM computed style
        // For now, return "visible" for both
        ("visible".to_string(), "visible".to_string())
    }
}
