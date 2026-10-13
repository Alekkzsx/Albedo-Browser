//! # Immutable Layout Fragment (`LayoutFragment`)
//!
//! Represents a finalized physical geometry fragment in the `FragmentTree`.
//! Fully immutable, thread-safe (`Arc<LayoutFragment>`), containing logical bounds,
//! snapped integer bounds, child fragments, and stacking context layer annotation.

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{LogicalPoint, LogicalRect};
use ace_style::computed::ComputedStyle;
use euclid::default::Rect;
use smol_str::SmolStr;
use std::sync::Arc;

use crate::fragment::overflow::LayoutOverflow;
use crate::fragment::pixel_snapping::snap_logical_rect;
use crate::positioning::stacking::StackingLayer;
use crate::tree::layout_box::LayoutBoxKind;

/// An immutable, thread-safe layout fragment produced by formatting contexts.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutFragment {
    /// Associated DOM / BoxTree NodeId.
    pub node_id: NodeId,
    /// Category of layout box that produced this fragment.
    pub box_kind: LayoutBoxKind,
    /// Border box rectangle in logical coordinates relative to its parent container.
    pub bounds: LogicalRect<LayoutUnit>,
    /// Content box rectangle in logical coordinates.
    pub content_rect: LogicalRect<LayoutUnit>,
    /// Subpixel snapped bounds in physical pixels ensuring zero pixel cracking.
    pub snapped_bounds: Rect<i32>,
    /// Child fragments within this fragment.
    pub children: Vec<Arc<LayoutFragment>>,
    /// Dual overflow tracking (scrollable and visual).
    pub overflow: LayoutOverflow,
    /// Canonical CSS 2.2 Appendix E painting layer (1 to 7).
    pub stacking_layer: StackingLayer,
    /// Resolved integer z-index.
    pub z_index: i32,
    /// Associated computed style.
    pub style: Arc<ComputedStyle>,
    /// Textual content if this fragment represents a text leaf.
    pub text: Option<SmolStr>,
    /// Optional typographic baseline offset from the top border.
    pub baseline: Option<LayoutUnit>,
}

impl LayoutFragment {
    /// Creates a standard box fragment.
    #[allow(clippy::too_many_arguments)]
    pub fn new_box(
        node_id: NodeId,
        box_kind: LayoutBoxKind,
        bounds: LogicalRect<LayoutUnit>,
        content_rect: LogicalRect<LayoutUnit>,
        children: Vec<Arc<LayoutFragment>>,
        overflow: LayoutOverflow,
        stacking_layer: StackingLayer,
        z_index: i32,
        style: Arc<ComputedStyle>,
        baseline: Option<LayoutUnit>,
    ) -> Self {
        let snapped_bounds = snap_logical_rect(&bounds);
        Self {
            node_id,
            box_kind,
            bounds,
            content_rect,
            snapped_bounds,
            children,
            overflow,
            stacking_layer,
            z_index,
            style,
            text: None,
            baseline,
        }
    }

    /// Creates a leaf text fragment.
    pub fn new_text(
        node_id: NodeId,
        bounds: LogicalRect<LayoutUnit>,
        text: SmolStr,
        stacking_layer: StackingLayer,
        style: Arc<ComputedStyle>,
        baseline: Option<LayoutUnit>,
    ) -> Self {
        let snapped_bounds = snap_logical_rect(&bounds);
        let overflow = LayoutOverflow::new(bounds);
        Self {
            node_id,
            box_kind: LayoutBoxKind::TextRun,
            bounds,
            content_rect: bounds,
            snapped_bounds,
            children: Vec::new(),
            overflow,
            stacking_layer,
            z_index: 0,
            style,
            text: Some(text),
            baseline,
        }
    }

    /// Returns the total count of fragments in this subtree including itself.
    pub fn total_fragments(&self) -> usize {
        1 + self
            .children
            .iter()
            .map(|c| c.total_fragments())
            .sum::<usize>()
    }

    /// Recursively finds a fragment by its DOM `NodeId`.
    pub fn find_by_node_id(self: &Arc<Self>, id: NodeId) -> Option<Arc<LayoutFragment>> {
        if self.node_id == id {
            return Some(Arc::clone(self));
        }
        for child in &self.children {
            if let Some(found) = child.find_by_node_id(id) {
                return Some(found);
            }
        }
        None
    }

    /// Checks if a logical point falls inside this fragment's border bounds.
    #[inline]
    pub fn contains_point(&self, point: LogicalPoint<LayoutUnit>) -> bool {
        let x_start = self.bounds.inline_start;
        let x_end = x_start.saturating_add(self.bounds.inline_size);
        let y_start = self.bounds.block_start;
        let y_end = y_start.saturating_add(self.bounds.block_size);

        point.inline >= x_start
            && point.inline <= x_end
            && point.block >= y_start
            && point.block <= y_end
    }

    /// Hit-tests a point against this subtree, returning the deepest intersecting NodeId.
    pub fn hit_test(&self, point: LogicalPoint<LayoutUnit>) -> Option<NodeId> {
        // Test children in reverse paint order (topmost first)
        for child in self.children.iter().rev() {
            let local_point = LogicalPoint {
                inline: point.inline.saturating_sub(self.bounds.inline_start),
                block: point.block.saturating_sub(self.bounds.block_start),
            };
            if let Some(hit) = child.hit_test(local_point) {
                return Some(hit);
            }
        }

        if self.contains_point(point) {
            Some(self.node_id)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ace_style::computed::ComputedStyle;

    #[test]
    fn test_layout_fragment_creation_and_counts() {
        let style = Arc::new(ComputedStyle::initial());
        let bounds = LogicalRect {
            inline_start: LayoutUnit::from_px(0),
            block_start: LayoutUnit::from_px(0),
            inline_size: LayoutUnit::from_px(100),
            block_size: LayoutUnit::from_px(100),
        };
        let overflow = LayoutOverflow::new(bounds);

        let child = Arc::new(LayoutFragment::new_text(
            NodeId::new(),
            LogicalRect {
                inline_start: LayoutUnit::from_px(10),
                block_start: LayoutUnit::from_px(10),
                inline_size: LayoutUnit::from_px(50),
                block_size: LayoutUnit::from_px(20),
            },
            SmolStr::new("hello"),
            StackingLayer::Layer5InlineContent,
            style.clone(),
            None,
        ));

        let root_id = NodeId::new();
        let root = Arc::new(LayoutFragment::new_box(
            root_id,
            LayoutBoxKind::Block,
            bounds,
            bounds,
            vec![child.clone()],
            overflow,
            StackingLayer::Layer1RootBackgroundAndBorders,
            0,
            style,
            None,
        ));

        assert_eq!(root.total_fragments(), 2);
        assert_eq!(root.find_by_node_id(root_id).unwrap().node_id, root_id);
        assert_eq!(
            root.find_by_node_id(child.node_id).unwrap().node_id,
            child.node_id
        );

        let hit = root.hit_test(LogicalPoint {
            inline: LayoutUnit::from_px(15),
            block: LayoutUnit::from_px(15),
        });
        assert_eq!(hit, Some(child.node_id));
    }
}
