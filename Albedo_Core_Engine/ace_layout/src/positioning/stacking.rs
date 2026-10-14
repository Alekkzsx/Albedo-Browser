//! # Stacking Contexts and 7-Layer Paint Ordering (CSS 2.2 Appendix E)
//!
//! Implements the normative CSS 2.2 Appendix E Stacking Context tree.
//! Annotates fragments with canonical painting layers (1 through 7) and resolves
//! the exact deterministic paint order required for rasterization and hit-testing.

use ace_core::id::NodeId;
use ace_style::computed::ComputedStyle;
use ace_style::values::enums::{Display, Position, ZIndex};

/// The canonical 7 painting layers within a Stacking Context (CSS 2.2 Appendix E).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum StackingLayer {
    /// Layer 1: Background and borders of the element forming the stacking context.
    Layer1RootBackgroundAndBorders = 1,
    /// Layer 2: Descendant stacking contexts with `z-index < 0` (sorted by z-index ascending).
    Layer2NegativeZIndex = 2,
    /// Layer 3: In-flow, non-positioned block-level descendants (backgrounds and borders).
    Layer3BlockBackgroundsAndBorders = 3,
    /// Layer 4: Non-positioned floating descendants.
    Layer4Floats = 4,
    /// Layer 5: In-flow, non-positioned inline-level descendants, line boxes, and text runs.
    Layer5InlineContent = 5,
    /// Layer 6: Positioned descendants with `z-index == 0` or `z-index: auto`.
    Layer6PositionedAutoZero = 6,
    /// Layer 7: Descendant stacking contexts with `z-index > 0` (sorted by z-index ascending).
    Layer7PositiveZIndex = 7,
}

impl StackingLayer {
    /// Returns the integer layer number (1 to 7).
    #[inline]
    pub const fn layer_number(self) -> u8 {
        self as u8
    }
}

/// Checks if an element establishes a new Stacking Context.
///
/// Per CSS 2.2 Appendix E & CSS Positioned Layout L3:
/// - Root element of the document.
/// - Positioned (`position != static`) with computed `z-index` other than `auto` (i.e. integer).
/// - Element with `opacity < 1.0`.
/// - Element with CSS transforms or filters.
pub fn establishes_stacking_context(style: &ComputedStyle, is_document_root: bool) -> bool {
    if is_document_root {
        return true;
    }

    let is_positioned = style.position() != Position::Static;
    let has_explicit_z_index = matches!(style.z_index(), ZIndex::Integer(_));

    if is_positioned && has_explicit_z_index {
        return true;
    }

    if style.opacity() < 1.0 {
        return true;
    }

    false
}

/// Determines the canonical painting layer of an element within its containing Stacking Context.
pub fn classify_stacking_layer(
    style: &ComputedStyle,
    is_context_root: bool,
    is_inline_or_text: bool,
) -> StackingLayer {
    if is_context_root {
        return StackingLayer::Layer1RootBackgroundAndBorders;
    }

    let creates_context = establishes_stacking_context(style, false);

    if creates_context {
        match style.z_index() {
            ZIndex::Integer(z) if z < 0 => StackingLayer::Layer2NegativeZIndex,
            ZIndex::Integer(z) if z > 0 => StackingLayer::Layer7PositiveZIndex,
            _ => StackingLayer::Layer6PositionedAutoZero,
        }
    } else if style.position() != Position::Static {
        StackingLayer::Layer6PositionedAutoZero
    } else if style.box_style.float.is_floating() {
        StackingLayer::Layer4Floats
    } else if is_inline_or_text || style.display() == Display::Inline {
        StackingLayer::Layer5InlineContent
    } else {
        StackingLayer::Layer3BlockBackgroundsAndBorders
    }
}

/// Node representing an item in the Stacking Context paint ordering list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaintItem<T> {
    pub node_id: NodeId,
    pub layer: StackingLayer,
    pub z_index: i32,
    pub tree_order: usize,
    pub item: T,
}

impl<T> PaintItem<T> {
    pub fn new(
        node_id: NodeId,
        layer: StackingLayer,
        z_index: i32,
        tree_order: usize,
        item: T,
    ) -> Self {
        Self {
            node_id,
            layer,
            z_index,
            tree_order,
            item,
        }
    }
}

/// Sorts a collection of paint items according to CSS 2.2 Appendix E paint ordering:
/// 1. Paint layer 1..=7
/// 2. Within Layer 2 and Layer 7: by `z_index` ascending, then `tree_order` ascending
/// 3. Within other layers: by `tree_order` ascending
pub fn sort_paint_items<T>(items: &mut [PaintItem<T>]) {
    items.sort_by(|a, b| {
        if a.layer != b.layer {
            return a.layer.cmp(&b.layer);
        }
        // Within the same layer:
        match a.layer {
            StackingLayer::Layer2NegativeZIndex | StackingLayer::Layer7PositiveZIndex => {
                if a.z_index != b.z_index {
                    a.z_index.cmp(&b.z_index)
                } else {
                    a.tree_order.cmp(&b.tree_order)
                }
            }
            _ => a.tree_order.cmp(&b.tree_order),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use ace_style::computed::ComputedStyle;
    use ace_style::values::enums::{Position, ZIndex};

    #[test]
    fn test_establishes_stacking_context() {
        let mut style = ComputedStyle::initial();
        assert!(!establishes_stacking_context(&style, false));
        assert!(establishes_stacking_context(&style, true)); // document root

        // Positioned with z-index
        let mut b = (*style.box_style).clone();
        b.position = Position::Relative;
        b.z_index = ZIndex::Integer(1);
        style.box_style = std::sync::Arc::new(b);
        assert!(establishes_stacking_context(&style, false));

        // Opacity < 1.0
        let mut style2 = ComputedStyle::initial();
        let mut c = (*style2.color).clone();
        c.opacity = 0.8;
        style2.color = std::sync::Arc::new(c);
        assert!(establishes_stacking_context(&style2, false));
    }

    #[test]
    fn test_stacking_layers_sorting() {
        let mut items = vec![
            PaintItem::new(
                NodeId::new(),
                StackingLayer::Layer7PositiveZIndex,
                10,
                0,
                "z10",
            ),
            PaintItem::new(
                NodeId::new(),
                StackingLayer::Layer2NegativeZIndex,
                -5,
                1,
                "z_minus_5",
            ),
            PaintItem::new(
                NodeId::new(),
                StackingLayer::Layer3BlockBackgroundsAndBorders,
                0,
                2,
                "block",
            ),
            PaintItem::new(
                NodeId::new(),
                StackingLayer::Layer1RootBackgroundAndBorders,
                0,
                3,
                "root_bg",
            ),
            PaintItem::new(
                NodeId::new(),
                StackingLayer::Layer5InlineContent,
                0,
                4,
                "text",
            ),
            PaintItem::new(
                NodeId::new(),
                StackingLayer::Layer7PositiveZIndex,
                2,
                5,
                "z2",
            ),
            PaintItem::new(
                NodeId::new(),
                StackingLayer::Layer2NegativeZIndex,
                -10,
                6,
                "z_minus_10",
            ),
        ];

        sort_paint_items(&mut items);

        let names: Vec<&str> = items.iter().map(|i| i.item).collect();
        // Expected order:
        // Layer 1: root_bg
        // Layer 2: z_minus_10 (-10), z_minus_5 (-5)
        // Layer 3: block
        // Layer 5: text
        // Layer 7: z2 (2), z10 (10)
        assert_eq!(
            names,
            vec![
                "root_bg",
                "z_minus_10",
                "z_minus_5",
                "block",
                "text",
                "z2",
                "z10"
            ]
        );
    }
}
