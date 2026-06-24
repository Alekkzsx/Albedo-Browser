use super::*;

use crate::ace::engine::dom::AceDOM;
use crate::ace::engine::layout::ElementGeometry;
use crate::ace::engine::core::AceEngine;




impl AceEngine {
    /// TODO: add docs
    pub fn layout(&mut self, width: f32, height: f32) {
        if let Some(ref dom_arc) = self.dom {
            let mut dom = dom_arc.lock().unwrap_or_else(|e| e.into_inner());
            let stylesheet = self.stylesheet.lock().unwrap_or_else(|e| e.into_inner());

            let mut taffy = self.taffy.lock().unwrap_or_else(|e| e.into_inner());
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
                        let mut geometry = self.element_geometry.lock().unwrap_or_else(|e| e.into_inner());
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
                        tracing::error!("Layout computation failed");
                    }
                }
            }
        }
    }
    /// TODO: add docs
    pub fn recompute_layout(&mut self) {
        let start_time = std::time::Instant::now();
        if let Some(ref dom_arc) = self.dom {
            let mut dom = dom_arc.lock().unwrap_or_else(|e| e.into_inner());
            let mut taffy = self.taffy.lock().unwrap_or_else(|e| e.into_inner());
            let stylesheet = self.stylesheet.lock().unwrap_or_else(|e| e.into_inner());

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
            tracing::debug!(
                total = ?total_duration,
                build = ?build_duration,
                compute = ?compute_duration,
                "Layout recomputed (incremental)"
            );
        }

        // 3.5. Float and Clear CSS apply pass
        self.apply_floats_and_clear();

        // 4. Projetar geometrias dos subframes para o espaço global
        self.collect_subframe_geometries();

        // 5. Sincronizar iframe_projected_geometry com o JsRuntime do frame pai
        if let Some(ref rt) = self.js_runtime {
            let projected = self.iframe_projected_geometry.lock().unwrap_or_else(|e| e.into_inner());
            let mut rt_projected = rt.iframe_projected_geometry.lock().unwrap_or_else(|e| e.into_inner());
            *rt_projected = projected.clone();
        }
    }
}
