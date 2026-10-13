//! # Out-of-Flow Fixed Positioning (`position: fixed`)
//!
//! Implements CSS 2.2 §9.7 viewport-anchored fixed positioning.
//! Fixed elements are positioned relative to the Initial Containing Block (ICB / Viewport),
//! remaining stationary across page scrolling.

use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::LogicalSize;
use ace_style::computed::ComputedStyle;

use crate::positioning::absolute::{
    resolve_absolute_position, AbsoluteLayoutResult, ContainingBlock,
};

/// Resolves the layout of a `position: fixed` element anchored to the viewport.
pub fn resolve_fixed_position(
    style: &ComputedStyle,
    viewport: LogicalSize<LayoutUnit>,
    intrinsic_width: LayoutUnit,
    intrinsic_height: LayoutUnit,
) -> AbsoluteLayoutResult {
    let cb = ContainingBlock::from_viewport(viewport);
    resolve_absolute_position(style, &cb, intrinsic_width, intrinsic_height)
}

/// Checks if an element establishes a containing block for `position: fixed` descendants.
///
/// Per CSS Transforms Module Level 1 and CSS Filter Effects:
/// Any ancestor with `transform`, `perspective`, or `filter` establishes a containing block
/// for fixed positioned descendants. In standard CSS without transforms, only the viewport does.
#[inline]
pub fn establishes_containing_block_for_fixed(_style: &ComputedStyle) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use ace_style::computed::ComputedStyle;
    use ace_style::values::enums::{Display, Position};

    #[test]
    fn test_fixed_positioning_viewport_anchored() {
        let viewport = LogicalSize::new(LayoutUnit::from_px(1024), LayoutUnit::from_px(768));
        let mut style = ComputedStyle::initial();
        let mut b = (*style.box_style).clone();
        b.display = Display::Block;
        b.position = Position::Fixed;
        b.top = Some(LayoutUnit::from_px(10));
        b.right = Some(LayoutUnit::from_px(20));
        b.width = Some(LayoutUnit::from_px(200));
        b.height = Some(LayoutUnit::from_px(50));
        style.box_style = std::sync::Arc::new(b);

        let res = resolve_fixed_position(
            &style,
            viewport,
            LayoutUnit::from_px(100),
            LayoutUnit::from_px(40),
        );

        assert_eq!(res.border_box.block_start, LayoutUnit::from_px(10));
        assert_eq!(res.border_box.inline_size, LayoutUnit::from_px(200));
        assert_eq!(res.border_box.block_size, LayoutUnit::from_px(50));
        // right = 20 -> x = 1024 - 20 - 200 = 804px
        assert_eq!(res.border_box.inline_start, LayoutUnit::from_px(804));
    }
}
