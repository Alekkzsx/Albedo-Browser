use super::*;

use crate::ace::engine::dom::AceDOM;
use crate::ace::engine::style::Stylesheet;
use crate::ace::engine::layout::ElementGeometry;
use crate::utils::time::unix_timestamp_secs_f64;
use taffy::geometry::MinMax;
use crate::ace::engine::core::AceEngine;



impl AceEngine {
    pub(crate) fn build_layout_tree(
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
            let node = dom.get_node(node_idx).expect("Albedo Engine: internal invariant violated");
            (node.node_type.clone(), node.children.clone())
        };
        let now = unix_timestamp_secs_f64();

        let style = {
            let engine_styles = self.element_styles.lock().unwrap_or_else(|e| e.into_inner());
            engine_styles.get(&node_idx).cloned().unwrap_or_else(|| {
                let am = self.animation_manager.lock().unwrap_or_else(|e| e.into_inner());
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
            let n2t = self.node_to_taffy.lock().unwrap_or_else(|e| e.into_inner());
            n2t.get(&node_idx).cloned()
        };

        let node_flags = {
            let node = dom.get_node(node_idx).expect("Albedo Engine: internal invariant violated");
            node.dirty
        };

        let is_strictly_dirty = node_flags.intersects(
            crate::ace::engine::dom::NodeDirtyFlags::LAYOUT
                | crate::ace::engine::dom::NodeDirtyFlags::CHILDREN
                | crate::ace::engine::dom::NodeDirtyFlags::STYLE,
        );
        let has_dirty_descendants =
            node_flags.contains(crate::ace::engine::dom::NodeDirtyFlags::SUBTREE);

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
            let mut geometry = self.element_geometry.lock().unwrap_or_else(|e| e.into_inner());
            let geom = geometry.entry(node_idx).or_insert(ElementGeometry::new());

            // Resolve Box Model
            let resolve = |l: &crate::ace::engine::style::css_values::CssLength| -> f32 {
                crate::ace::engine::style::css_values::resolve_length(l, style.font_size, 16.0, vw, vh)
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
            .contains(&crate::ace::engine::style::css_values::CssLength::Subgrid);
        let is_subgrid_rows = style
            .grid_template_rows
            .contains(&crate::ace::engine::style::css_values::CssLength::Subgrid);
        let should_flatten = style.display
            == crate::ace::engine::style::css_values::CssDisplay::Contents
            || is_subgrid_cols
            || is_subgrid_rows;

        let mut grid_ctx = None;

        if style.display == crate::ace::engine::style::css_values::CssDisplay::Grid {
            grid_ctx = Some(GridContext {
                column_names: self.resolve_grid_names(&style.grid_template_columns),
                row_names: self.resolve_grid_names(&style.grid_template_rows),
                areas: self.resolve_grid_areas(&style.grid_template_areas),
                col_offset: 0,
                row_offset: 0,
            });
        } else if style.display == crate::ace::engine::style::css_values::CssDisplay::Table {
            include!("layout_table.rs");
        } else if (is_subgrid_cols || is_subgrid_rows) && parent_grid_ctx.is_some() {
            include!("layout_subgrid.rs");
        } else if let crate::ace::engine::dom::AceNodeType::Element(el) = &node_type {
            include!("layout_iframe.rs");
            } else if el.tag == "svg" {
            include!("layout_svg.rs");
        } else if let crate::ace::engine::dom::AceNodeType::Text(text) = &node_type {
            include!("layout_text.rs");

        let mut children = Vec::new();

        include!("layout_children.rs");

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
                let dom_node = dom.get_node(node_idx).expect("Albedo Engine: internal invariant violated");
                dom_node.dirty.intersects(
                    crate::ace::engine::dom::NodeDirtyFlags::CHILDREN
                        | crate::ace::engine::dom::NodeDirtyFlags::LAYOUT,
                )
            };
            if is_children_dirty || taffy.children(node).unwrap_or_default().len() != children.len()
            {
                let _ = taffy.set_children(node, &children);
            }
            node
        } else {
            let node = taffy.new_with_children(taffy_style, &children).expect("Albedo Engine: internal invariant violated");
            self.node_to_taffy.lock().unwrap_or_else(|e| e.into_inner()).insert(node_idx, node);
            node
        };

        node_map.insert(taffy_node, node_idx);
        vec![taffy_node]
    }
}
