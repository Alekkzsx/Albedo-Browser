
use crate::ace::engine::core::AceEngine;

impl AceEngine {
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
    pub fn scroll_into_view(&mut self, node_idx: usize) {
        let geometry = self.element_geometry.lock().unwrap();
        if let Some(geom) = geometry.get(&node_idx) {
            // No Slint, viewport-y é 0 no topo e fica mais negativo à medida que descemos.
            // Para colocar o elemento no topo da visão: viewport_y = -y
            // Para centralizar: viewport_y = -y + (window_height / 2)
            self.viewport_y = -geom.y;
            tracing::debug!(node_idx, y = geom.y, "Scrolling to node");
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
}
