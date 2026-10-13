//! # Dual Overflow Calculation (`LayoutOverflow`)
//!
//! Tracks both scrollable overflow (scroll limits, `scrollWidth`/`scrollHeight`)
//! and visual overflow (paint bounds, culling, shadows, outlines).

use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::LogicalRect;

/// Dual overflow descriptor representing both scrollable and visual bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutOverflow {
    /// Geometric bounds determining scroll bars and scrollable area (`scrollWidth`/`scrollHeight`).
    pub scrollable_overflow: LogicalRect<LayoutUnit>,
    /// Expanded bounds determining rasterizer culling and paint clipping (including shadows, outlines).
    pub visual_overflow: LogicalRect<LayoutUnit>,
}

impl LayoutOverflow {
    /// Creates a new `LayoutOverflow` initialized to the given border box rectangle.
    #[inline]
    pub fn new(border_box: LogicalRect<LayoutUnit>) -> Self {
        Self {
            scrollable_overflow: border_box,
            visual_overflow: border_box,
        }
    }

    /// Creates an empty `LayoutOverflow` with zero-sized rectangles.
    #[inline]
    pub fn zero() -> Self {
        let zero_rect = LogicalRect {
            inline_start: LayoutUnit::ZERO,
            block_start: LayoutUnit::ZERO,
            inline_size: LayoutUnit::ZERO,
            block_size: LayoutUnit::ZERO,
        };
        Self {
            scrollable_overflow: zero_rect,
            visual_overflow: zero_rect,
        }
    }

    /// Computes the union with another `LayoutOverflow`.
    pub fn union(&mut self, other: &LayoutOverflow) {
        self.scrollable_overflow =
            union_logical_rects(&self.scrollable_overflow, &other.scrollable_overflow);
        self.visual_overflow = union_logical_rects(&self.visual_overflow, &other.visual_overflow);
    }

    /// Unites the scrollable overflow with a given rectangle.
    #[inline]
    pub fn union_scrollable(&mut self, rect: &LogicalRect<LayoutUnit>) {
        self.scrollable_overflow = union_logical_rects(&self.scrollable_overflow, rect);
    }

    /// Unites the visual overflow with a given rectangle.
    #[inline]
    pub fn union_visual(&mut self, rect: &LogicalRect<LayoutUnit>) {
        self.visual_overflow = union_logical_rects(&self.visual_overflow, rect);
    }
}

/// Helper calculating the minimum bounding box encompassing both logical rectangles.
pub fn union_logical_rects(
    a: &LogicalRect<LayoutUnit>,
    b: &LogicalRect<LayoutUnit>,
) -> LogicalRect<LayoutUnit> {
    if a.inline_size == LayoutUnit::ZERO && a.block_size == LayoutUnit::ZERO {
        return *b;
    }
    if b.inline_size == LayoutUnit::ZERO && b.block_size == LayoutUnit::ZERO {
        return *a;
    }

    let a_inline_end = a.inline_start.saturating_add(a.inline_size);
    let a_block_end = a.block_start.saturating_add(a.block_size);

    let b_inline_end = b.inline_start.saturating_add(b.inline_size);
    let b_block_end = b.block_start.saturating_add(b.block_size);

    let min_inline = a.inline_start.min(b.inline_start);
    let min_block = a.block_start.min(b.block_start);

    let max_inline = a_inline_end.max(b_inline_end);
    let max_block = a_block_end.max(b_block_end);

    LogicalRect {
        inline_start: min_inline,
        block_start: min_block,
        inline_size: max_inline.saturating_sub(min_inline),
        block_size: max_block.saturating_sub(min_block),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_overflow_union() {
        let r1 = LogicalRect {
            inline_start: LayoutUnit::from_px(0),
            block_start: LayoutUnit::from_px(0),
            inline_size: LayoutUnit::from_px(100),
            block_size: LayoutUnit::from_px(100),
        };
        let mut overflow = LayoutOverflow::new(r1);

        let r2 = LogicalRect {
            inline_start: LayoutUnit::from_px(50),
            block_start: LayoutUnit::from_px(50),
            inline_size: LayoutUnit::from_px(100),
            block_size: LayoutUnit::from_px(100),
        };
        overflow.union_scrollable(&r2);

        assert_eq!(
            overflow.scrollable_overflow.inline_start,
            LayoutUnit::from_px(0)
        );
        assert_eq!(
            overflow.scrollable_overflow.block_start,
            LayoutUnit::from_px(0)
        );
        assert_eq!(
            overflow.scrollable_overflow.inline_size,
            LayoutUnit::from_px(150)
        );
        assert_eq!(
            overflow.scrollable_overflow.block_size,
            LayoutUnit::from_px(150)
        );
    }
}
