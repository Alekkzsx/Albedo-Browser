//! # Deterministic Textual Layout Dump Generator (`LayoutDump`)
//!
//! Generates canonical, fixed-point textual representations of `FragmentTree` and
//! `LayoutFragment` graphs for regression testing, visual inspection, and snapshot testing.

use crate::fragment::fragment_tree::FragmentTree;
use crate::fragment::layout_fragment::LayoutFragment;
use crate::tree::layout_box::LayoutBoxKind;
use std::fmt::Write;

/// Textual layout dump generator.
pub struct LayoutDump;

impl LayoutDump {
    /// Formats an entire `FragmentTree` into a canonical deterministic string.
    pub fn dump_fragment_tree(tree: &FragmentTree) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "FragmentTree viewport: {:.2}x{:.2} fragments: {}",
            tree.viewport.inline_size.to_f32_px(),
            tree.viewport.block_size.to_f32_px(),
            tree.total_fragments()
        );

        if let Some(ref root) = tree.root {
            Self::dump_fragment_recursive(root, 0, &mut out);
        } else {
            out.push_str("  <Empty>\n");
        }

        out
    }

    /// Formats a single `LayoutFragment` and its descendants into a string.
    pub fn dump_fragment(fragment: &LayoutFragment, indent: usize) -> String {
        let mut out = String::new();
        Self::dump_fragment_recursive(fragment, indent, &mut out);
        out
    }

    fn dump_fragment_recursive(fragment: &LayoutFragment, indent: usize, out: &mut String) {
        let pad = "  ".repeat(indent);

        let bounds = &fragment.bounds;
        let content = &fragment.content_rect;

        let b_x = bounds.inline_start.to_f32_px();
        let b_y = bounds.block_start.to_f32_px();
        let b_w = bounds.inline_size.to_f32_px();
        let b_h = bounds.block_size.to_f32_px();

        let c_x = content.inline_start.to_f32_px();
        let c_y = content.block_start.to_f32_px();
        let c_w = content.inline_size.to_f32_px();
        let c_h = content.block_size.to_f32_px();

        match fragment.box_kind {
            LayoutBoxKind::TextRun => {
                let text = fragment.text.as_deref().unwrap_or("");
                let _ = writeln!(
                    out,
                    "{}[TextFragment \"{}\"] rect: {:.2},{:.2} {:.2}x{:.2} layer: {}",
                    pad,
                    text,
                    b_x,
                    b_y,
                    b_w,
                    b_h,
                    fragment.stacking_layer.layer_number()
                );
            }
            kind => {
                let _ = writeln!(
                    out,
                    "{}[BoxFragment {:?} #{}] rect: {:.2},{:.2} {:.2}x{:.2} content: {:.2},{:.2} {:.2}x{:.2} layer: {}{}",
                    pad,
                    kind,
                    fragment.node_id.raw(),
                    b_x,
                    b_y,
                    b_w,
                    b_h,
                    c_x,
                    c_y,
                    c_w,
                    c_h,
                    fragment.stacking_layer.layer_number(),
                    if fragment.z_index != 0 {
                        format!(" z: {}", fragment.z_index)
                    } else {
                        String::new()
                    }
                );
            }
        }

        for child in &fragment.children {
            Self::dump_fragment_recursive(child, indent + 1, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fragment::overflow::LayoutOverflow;
    use crate::positioning::stacking::StackingLayer;
    use ace_core::id::NodeId;
    use ace_core::math::layout_unit::LayoutUnit;
    use ace_core::math::logical::{LogicalRect, LogicalSize};
    use ace_style::computed::ComputedStyle;
    use smol_str::SmolStr;
    use std::sync::Arc;

    #[test]
    fn test_layout_dump_generation() {
        let style = Arc::new(ComputedStyle::initial());
        let bounds = LogicalRect {
            inline_start: LayoutUnit::from_px(0),
            block_start: LayoutUnit::from_px(0),
            inline_size: LayoutUnit::from_px(800),
            block_size: LayoutUnit::from_px(600),
        };
        let text_bounds = LogicalRect {
            inline_start: LayoutUnit::from_px(10),
            block_start: LayoutUnit::from_px(10),
            inline_size: LayoutUnit::from_px(100),
            block_size: LayoutUnit::from_px(20),
        };

        let text_frag = Arc::new(LayoutFragment::new_text(
            NodeId::new(),
            text_bounds,
            SmolStr::new("Albedo Engine"),
            StackingLayer::Layer5InlineContent,
            style.clone(),
            None,
        ));

        let root_frag = Arc::new(LayoutFragment::new_box(
            NodeId::new(),
            LayoutBoxKind::Block,
            bounds,
            bounds,
            vec![text_frag],
            LayoutOverflow::new(bounds),
            StackingLayer::Layer1RootBackgroundAndBorders,
            0,
            style,
            None,
        ));

        let tree = FragmentTree::new(
            Some(root_frag),
            LogicalSize::new(LayoutUnit::from_px(800), LayoutUnit::from_px(600)),
        );
        let dump_str = LayoutDump::dump_fragment_tree(&tree);

        assert!(dump_str.contains("FragmentTree viewport: 800.00x600.00"));
        assert!(dump_str.contains("[BoxFragment Block"));
        assert!(dump_str.contains("[TextFragment \"Albedo Engine\"]"));
    }
}
