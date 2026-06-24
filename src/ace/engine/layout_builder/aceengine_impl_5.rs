use super::*;

use crate::ace::engine::dom::AceDOM;
use crate::ace::engine::style::Stylesheet;
use crate::ace::engine::layout::ElementGeometry;
use crate::utils::time::unix_timestamp_secs_f64;
use taffy::geometry::MinMax;
use crate::ace::engine::core::AceEngine;



impl AceEngine {
    pub(crate) fn _to_taffy_length_percentage_auto(
        &self,
        len: &crate::ace::engine::style::css_values::CssLength,
    ) -> taffy::prelude::LengthPercentageAuto {
        use crate::ace::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::LengthPercentageAuto::Points(*v),
            CssLength::Percent(v) => taffy::prelude::LengthPercentageAuto::Percent(*v / 100.0),
            CssLength::Auto => taffy::prelude::LengthPercentageAuto::Auto,
            _ => taffy::prelude::LengthPercentageAuto::Points(0.0),
        }
    }
    pub(crate) fn resolve_placement(
        &self,
        placement: &crate::ace::engine::style::css_values::CssLength,
    ) -> taffy::prelude::GridPlacement {
        use crate::ace::engine::style::css_values::CssLength;
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
    pub(crate) fn populate_node_map_recursively(
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
}
