use super::*;

use crate::ace::engine::dom::AceDOM;
use crate::ace::engine::layout::ElementGeometry;
use crate::ace::engine::core::AceEngine;




impl AceEngine {
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
        let mut projected = self.iframe_projected_geometry.lock().unwrap_or_else(|e| e.into_inner());
        projected.clear();

        let dom_arc = match &self.dom {
            Some(d) => d,
            None => return,
        };

        let dom = dom_arc.lock().unwrap_or_else(|e| e.into_inner());
        let subframes_arc = match &dom.subframes {
            Some(s) => s.clone(),
            None => return,
        };
        // Drop DOM lock before iterating subframes (evitar deadlock)
        drop(dom);

        let subframes = subframes_arc.lock().unwrap_or_else(|e| e.into_inner());
        let parent_geometry = self.element_geometry.lock().unwrap_or_else(|e| e.into_inner());

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
            let sub_engine = sub_engine_arc.lock().unwrap_or_else(|e| e.into_inner());
            let sub_geometry = sub_engine.element_geometry.lock().unwrap_or_else(|e| e.into_inner());

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
}
