//! # Immutable Fragment Tree (`FragmentTree`)
//!
//! Root of the finalized layout geometry graph. Contains the root fragment,
//! viewport dimensions, precomputed 7-layer paint ordering list, and hit-testing helpers.

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{LogicalPoint, LogicalSize};
use std::sync::Arc;

use crate::fragment::layout_fragment::LayoutFragment;
use crate::positioning::stacking::{sort_paint_items, PaintItem};

/// The finalized, immutable layout fragment tree for a document.
#[derive(Debug, Clone, PartialEq)]
pub struct FragmentTree {
    /// Root fragment of the document (typically `<html>` / document element).
    pub root: Option<Arc<LayoutFragment>>,
    /// Viewport dimensions used to compute layout.
    pub viewport: LogicalSize<LayoutUnit>,
    /// Precomputed paint-ordered sequence of fragments (Layers 1..=7).
    pub paint_order: Vec<Arc<LayoutFragment>>,
}

impl FragmentTree {
    /// Creates a new `FragmentTree` and builds the canonical 7-layer paint order list.
    pub fn new(root: Option<Arc<LayoutFragment>>, viewport: LogicalSize<LayoutUnit>) -> Self {
        let mut paint_order = Vec::new();
        if let Some(ref r) = root {
            let mut items = Vec::new();
            collect_paint_items(r, &mut items, &mut 0);
            sort_paint_items(&mut items);
            paint_order = items.into_iter().map(|p| p.item).collect();
        }

        Self {
            root,
            viewport,
            paint_order,
        }
    }

    /// Creates an empty `FragmentTree`.
    #[inline]
    pub fn empty(viewport: LogicalSize<LayoutUnit>) -> Self {
        Self {
            root: None,
            viewport,
            paint_order: Vec::new(),
        }
    }

    /// Returns a reference to the root fragment if present.
    #[inline]
    pub fn root(&self) -> Option<&Arc<LayoutFragment>> {
        self.root.as_ref()
    }

    /// Viewport dimensions.
    #[inline]
    pub fn viewport(&self) -> LogicalSize<LayoutUnit> {
        self.viewport
    }

    /// Returns the total count of fragments in the entire tree.
    #[inline]
    pub fn total_fragments(&self) -> usize {
        self.root.as_ref().map(|r| r.total_fragments()).unwrap_or(0)
    }

    /// Returns the canonical paint-ordered list of fragments.
    #[inline]
    pub fn paint_order(&self) -> &[Arc<LayoutFragment>] {
        &self.paint_order
    }

    /// Finds a fragment by its DOM NodeId.
    pub fn find_by_node_id(&self, id: NodeId) -> Option<Arc<LayoutFragment>> {
        self.root.as_ref().and_then(|r| r.find_by_node_id(id))
    }

    /// Performs hit-testing at a point in viewport coordinates.
    /// Evaluates fragments in reverse paint order so topmost elements receive hits first.
    pub fn hit_test(&self, point: LogicalPoint<LayoutUnit>) -> Option<NodeId> {
        for fragment in self.paint_order.iter().rev() {
            if fragment.contains_point(point) {
                return Some(fragment.node_id);
            }
        }
        None
    }
}

/// Helper recursively gathering fragments with their tree order for Stacking Context sorting.
fn collect_paint_items(
    fragment: &Arc<LayoutFragment>,
    items: &mut Vec<PaintItem<Arc<LayoutFragment>>>,
    counter: &mut usize,
) {
    let order = *counter;
    *counter += 1;

    items.push(PaintItem::new(
        fragment.node_id,
        fragment.stacking_layer,
        fragment.z_index,
        order,
        Arc::clone(fragment),
    ));

    for child in &fragment.children {
        collect_paint_items(child, items, counter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fragment::overflow::LayoutOverflow;
    use crate::positioning::stacking::StackingLayer;
    use crate::tree::layout_box::LayoutBoxKind;
    use ace_core::math::logical::LogicalRect;
    use ace_style::computed::ComputedStyle;

    #[test]
    fn test_fragment_tree_paint_order_and_hit_test() {
        let style = Arc::new(ComputedStyle::initial());
        let viewport = LogicalSize::new(LayoutUnit::from_px(800), LayoutUnit::from_px(600));

        let bg_rect = LogicalRect {
            inline_start: LayoutUnit::from_px(0),
            block_start: LayoutUnit::from_px(0),
            inline_size: LayoutUnit::from_px(400),
            block_size: LayoutUnit::from_px(300),
        };
        let fg_rect = LogicalRect {
            inline_start: LayoutUnit::from_px(50),
            block_start: LayoutUnit::from_px(50),
            inline_size: LayoutUnit::from_px(100),
            block_size: LayoutUnit::from_px(100),
        };

        let fg_id = NodeId::new();
        let fg = Arc::new(LayoutFragment::new_box(
            fg_id,
            LayoutBoxKind::Block,
            fg_rect,
            fg_rect,
            Vec::new(),
            LayoutOverflow::new(fg_rect),
            StackingLayer::Layer7PositiveZIndex,
            1,
            style.clone(),
            None,
        ));

        let bg_id = NodeId::new();
        let bg = Arc::new(LayoutFragment::new_box(
            bg_id,
            LayoutBoxKind::Block,
            bg_rect,
            bg_rect,
            vec![fg.clone()],
            LayoutOverflow::new(bg_rect),
            StackingLayer::Layer1RootBackgroundAndBorders,
            0,
            style,
            None,
        ));

        let tree = FragmentTree::new(Some(bg), viewport);
        assert_eq!(tree.total_fragments(), 2);
        assert_eq!(tree.paint_order().len(), 2);
        assert_eq!(tree.paint_order()[0].node_id, bg_id); // Layer 1 paints first
        assert_eq!(tree.paint_order()[1].node_id, fg_id); // Layer 7 paints on top

        // Hit testing over the overlap region (60, 60) should hit the topmost fragment (fg)
        let hit = tree.hit_test(LogicalPoint {
            inline: LayoutUnit::from_px(60),
            block: LayoutUnit::from_px(60),
        });
        assert_eq!(hit, Some(fg_id));

        // Hit testing at (20, 20) hits bg
        let hit_bg = tree.hit_test(LogicalPoint {
            inline: LayoutUnit::from_px(20),
            block: LayoutUnit::from_px(20),
        });
        assert_eq!(hit_bg, Some(bg_id));
    }
}
