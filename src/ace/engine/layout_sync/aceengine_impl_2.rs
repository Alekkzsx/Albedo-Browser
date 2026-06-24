use super::*;

use crate::ace::engine::dom::AceDOM;
use crate::ace::engine::layout::ElementGeometry;
use crate::ace::engine::core::AceEngine;




impl AceEngine {
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
            let pos_x = parent_x + layout.location.x;
            let pos_y = parent_y + layout.location.y;
            let w = layout.size.width;
            let h = layout.size.height;

            if let Some(&dom_idx) = node_map.get(&node) {
                let mut geom = ElementGeometry::new();
                geom.pos_x = pos_x;
                geom.pos_y = pos_y;
                geom.width = w;
                geom.height = h;
                // TODO: Extract borders, padding, content dimensions, overflow styles from DOM
                geometry.insert(dom_idx, geom);
            }

            if let Ok(children) = taffy.children(node) {
                for child in children {
                    self.extract_layout_recursively(taffy, child, node_map, geometry, pos_x, pos_y);
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
        let layout = taffy.layout(taffy_node).expect("Albedo Engine: internal invariant violated");
        let abs_x = offset_x + layout.location.x;
        let abs_y = offset_y + layout.location.y;
        let width = layout.size.width;
        let height = layout.size.height;

        let mut content_w = 0.0f32;
        let mut content_h = 0.0f32;

        // Calculate content dimensions from children
        let taffy_children = taffy.children(taffy_node).expect("Albedo Engine: internal invariant violated");
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
            let mut geometry = self.element_geometry.lock().unwrap_or_else(|e| e.into_inner());
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
            let scroll_map = self.element_scroll.lock().unwrap_or_else(|e| e.into_inner());
            scroll_map.get(&node_idx).copied().unwrap_or((0.0, 0.0))
        } else {
            (0.0, 0.0)
        };

        for &taffy_child in &taffy_children {
            self.sync_taffy_bounds(taffy, dom, taffy_child, abs_x - sx, abs_y - sy, node_map);
        }
    }
    /// TODO: add docs
    pub fn apply_floats_and_clear(&self) {
        let dom_arc = match &self.dom {
            Some(d) => d.clone(),
            None => return,
        };
        let dom = dom_arc.lock().unwrap_or_else(|e| e.into_inner());

        let mut float_ctx = FloatContext::default();
        let mut geometry = self.element_geometry.lock().unwrap_or_else(|e| e.into_inner());
        self.apply_floats_recursive(0, &dom, &mut float_ctx, &mut geometry, 0.0);
    }
}
