//! # Out-of-Flow Absolute Positioning (`position: absolute`)
//!
//! Implements CSS 2.2 §9.7, §10.3.7, and §10.6.4 out-of-flow absolute positioning.
//! Resolves containing blocks, inset offsets (`top`, `right`, `bottom`, `left`),
//! box sizing models, and automatic margin distribution (`margin: auto`) for centering.

use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{LogicalRect, LogicalSides, LogicalSize};
use ace_style::computed::ComputedStyle;
use ace_style::values::enums::{BoxSizing, Position};

/// Containing block context for positioned layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainingBlock {
    /// Padding box rectangle of the containing block in layout coordinates.
    pub rect: LogicalRect<LayoutUnit>,
    /// Indicates whether this containing block represents the initial containing block (viewport).
    pub is_viewport: bool,
}

impl ContainingBlock {
    /// Creates a containing block from a padding box rectangle.
    #[inline]
    pub const fn new(rect: LogicalRect<LayoutUnit>, is_viewport: bool) -> Self {
        Self { rect, is_viewport }
    }

    /// Creates an initial containing block from viewport dimensions.
    #[inline]
    pub const fn from_viewport(viewport: LogicalSize<LayoutUnit>) -> Self {
        Self {
            rect: LogicalRect {
                inline_start: LayoutUnit::ZERO,
                block_start: LayoutUnit::ZERO,
                inline_size: viewport.inline_size,
                block_size: viewport.block_size,
            },
            is_viewport: true,
        }
    }

    /// Available inline size (width in horizontal writing mode).
    #[inline]
    pub fn inline_size(&self) -> LayoutUnit {
        self.rect.inline_size
    }

    /// Available block size (height in horizontal writing mode).
    #[inline]
    pub fn block_size(&self) -> LayoutUnit {
        self.rect.block_size
    }
}

/// Resolved insets for an absolutely positioned box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResolvedInsets {
    pub top: LayoutUnit,
    pub right: LayoutUnit,
    pub bottom: LayoutUnit,
    pub left: LayoutUnit,
}

/// Result of resolving out-of-flow absolute positioning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsoluteLayoutResult {
    /// Border box in coordinates relative to the containing block's origin.
    pub border_box: LogicalRect<LayoutUnit>,
    /// Content box in coordinates relative to the containing block's origin.
    pub content_box: LogicalRect<LayoutUnit>,
    /// Padding box in coordinates relative to the containing block's origin.
    pub padding_box: LogicalRect<LayoutUnit>,
    /// Margin box in coordinates relative to the containing block's origin.
    pub margin_box: LogicalRect<LayoutUnit>,
    /// Resolved margins on all four sides.
    pub margins: LogicalSides<LayoutUnit>,
    /// Resolved insets.
    pub insets: ResolvedInsets,
}

/// Checks if an element establishes a containing block for `position: absolute` descendants.
///
/// Per CSS 2.2 §10.1: For absolute elements, the containing block is established by the
/// nearest ancestor with `position != static` (or `transform`, `filter`, etc.).
#[inline]
pub fn establishes_containing_block_for_absolute(style: &ComputedStyle) -> bool {
    style.position() != Position::Static
}

/// Resolves out-of-flow absolute positioning for a box given its style, intrinsic sizes,
/// and containing block.
pub fn resolve_absolute_position(
    style: &ComputedStyle,
    cb: &ContainingBlock,
    intrinsic_width: LayoutUnit,
    intrinsic_height: LayoutUnit,
) -> AbsoluteLayoutResult {
    let cb_width = cb.inline_size().max(LayoutUnit::ZERO);
    let cb_height = cb.block_size().max(LayoutUnit::ZERO);

    let padding_left = style.padding_left();
    let padding_right = style.padding_right();
    let padding_top = style.padding_top();
    let padding_bottom = style.padding_bottom();

    let border_left = style.border_width().left;
    let border_right = style.border_width().right;
    let border_top = style.border_width().top;
    let border_bottom = style.border_width().bottom;

    let non_content_h = padding_left
        .saturating_add(padding_right)
        .saturating_add(border_left)
        .saturating_add(border_right);

    let non_content_v = padding_top
        .saturating_add(padding_bottom)
        .saturating_add(border_top)
        .saturating_add(border_bottom);

    let box_sizing = style.box_sizing();

    // -------------------------------------------------------------------------
    // Horizontal Sizing & Positioning (CSS 2.2 §10.3.7)
    // -------------------------------------------------------------------------
    let left_opt = style.left();
    let right_opt = style.right();
    let width_opt = style.width().map(|w| match box_sizing {
        BoxSizing::BorderBox => w.saturating_sub(non_content_h).max(LayoutUnit::ZERO),
        BoxSizing::ContentBox => w.max(LayoutUnit::ZERO),
    });

    let margin_left_opt = style.margin_left();
    let margin_right_opt = style.margin_right();

    let min_width = style.min_width().map(|w| match box_sizing {
        BoxSizing::BorderBox => w.saturating_sub(non_content_h).max(LayoutUnit::ZERO),
        BoxSizing::ContentBox => w.max(LayoutUnit::ZERO),
    });
    let max_width = style.max_width().map(|w| match box_sizing {
        BoxSizing::BorderBox => w.saturating_sub(non_content_h).max(LayoutUnit::ZERO),
        BoxSizing::ContentBox => w.max(LayoutUnit::ZERO),
    });

    let (resolved_left, resolved_width, resolved_margin_left, resolved_margin_right) =
        resolve_1d_axis(
            cb_width,
            left_opt,
            right_opt,
            width_opt,
            margin_left_opt,
            margin_right_opt,
            intrinsic_width,
            non_content_h,
            min_width,
            max_width,
        );

    // -------------------------------------------------------------------------
    // Vertical Sizing & Positioning (CSS 2.2 §10.6.4)
    // -------------------------------------------------------------------------
    let top_opt = style.top();
    let bottom_opt = style.bottom();
    let height_opt = style.height().map(|h| match box_sizing {
        BoxSizing::BorderBox => h.saturating_sub(non_content_v).max(LayoutUnit::ZERO),
        BoxSizing::ContentBox => h.max(LayoutUnit::ZERO),
    });

    let margin_top_opt = style.margin_top();
    let margin_bottom_opt = style.margin_bottom();

    let min_height = style.min_height().map(|h| match box_sizing {
        BoxSizing::BorderBox => h.saturating_sub(non_content_v).max(LayoutUnit::ZERO),
        BoxSizing::ContentBox => h.max(LayoutUnit::ZERO),
    });
    let max_height = style.max_height().map(|h| match box_sizing {
        BoxSizing::BorderBox => h.saturating_sub(non_content_v).max(LayoutUnit::ZERO),
        BoxSizing::ContentBox => h.max(LayoutUnit::ZERO),
    });

    let (resolved_top, resolved_height, resolved_margin_top, resolved_margin_bottom) =
        resolve_1d_axis(
            cb_height,
            top_opt,
            bottom_opt,
            height_opt,
            margin_top_opt,
            margin_bottom_opt,
            intrinsic_height,
            non_content_v,
            min_height,
            max_height,
        );

    // Assemble boxes relative to the containing block padding box origin
    let border_x = resolved_left.saturating_add(resolved_margin_left);
    let border_y = resolved_top.saturating_add(resolved_margin_top);
    let border_w = resolved_width.saturating_add(non_content_h);
    let border_h = resolved_height.saturating_add(non_content_v);

    let border_box = LogicalRect {
        inline_start: border_x,
        block_start: border_y,
        inline_size: border_w,
        block_size: border_h,
    };

    let padding_box = LogicalRect {
        inline_start: border_x.saturating_add(border_left),
        block_start: border_y.saturating_add(border_top),
        inline_size: border_w.saturating_sub(border_left.saturating_add(border_right)),
        block_size: border_h.saturating_sub(border_top.saturating_add(border_bottom)),
    };

    let content_box = LogicalRect {
        inline_start: padding_box.inline_start.saturating_add(padding_left),
        block_start: padding_box.block_start.saturating_add(padding_top),
        inline_size: resolved_width,
        block_size: resolved_height,
    };

    let margin_box = LogicalRect {
        inline_start: resolved_left,
        block_start: resolved_top,
        inline_size: border_w
            .saturating_add(resolved_margin_left)
            .saturating_add(resolved_margin_right),
        block_size: border_h
            .saturating_add(resolved_margin_top)
            .saturating_add(resolved_margin_bottom),
    };

    let margins = LogicalSides::new(
        resolved_margin_left,
        resolved_margin_right,
        resolved_margin_top,
        resolved_margin_bottom,
    );

    let insets = ResolvedInsets {
        top: resolved_top,
        right: cb_width
            .saturating_sub(border_x)
            .saturating_sub(border_w)
            .saturating_sub(resolved_margin_right),
        bottom: cb_height
            .saturating_sub(border_y)
            .saturating_sub(border_h)
            .saturating_sub(resolved_margin_bottom),
        left: resolved_left,
    };

    AbsoluteLayoutResult {
        border_box,
        content_box,
        padding_box,
        margin_box,
        margins,
        insets,
    }
}

/// Generic 1D axis resolver implementing the CSS 2.2 §10.3.7 / §10.6.4 constraints.
/// Returns `(start_offset, content_size, start_margin, end_margin)`.
#[allow(clippy::too_many_arguments)]
fn resolve_1d_axis(
    cb_size: LayoutUnit,
    start_opt: Option<LayoutUnit>,
    end_opt: Option<LayoutUnit>,
    size_opt: Option<LayoutUnit>,
    margin_start_opt: Option<LayoutUnit>,
    margin_end_opt: Option<LayoutUnit>,
    intrinsic_size: LayoutUnit,
    non_content: LayoutUnit,
    min_size: Option<LayoutUnit>,
    max_size: Option<LayoutUnit>,
) -> (LayoutUnit, LayoutUnit, LayoutUnit, LayoutUnit) {
    let mut margin_start = margin_start_opt.unwrap_or(LayoutUnit::ZERO);
    let mut margin_end = margin_end_opt.unwrap_or(LayoutUnit::ZERO);

    let is_margin_start_auto = margin_start_opt.is_none();
    let is_margin_end_auto = margin_end_opt.is_none();

    let (start, content_size) = match (start_opt, end_opt, size_opt) {
        // Case 1: All three are auto
        (None, None, None) => {
            let mut sz = intrinsic_size;
            if let Some(min) = min_size {
                sz = sz.max(min);
            }
            if let Some(max) = max_size {
                sz = sz.min(max);
            }
            (LayoutUnit::ZERO, sz)
        }
        // Case 2: start and end are specified, size is auto
        (Some(start_val), Some(end_val), None) => {
            let available = cb_size
                .saturating_sub(start_val)
                .saturating_sub(end_val)
                .saturating_sub(non_content);
            let mut sz = if is_margin_start_auto && is_margin_end_auto {
                available.max(LayoutUnit::ZERO)
            } else {
                available
                    .saturating_sub(margin_start)
                    .saturating_sub(margin_end)
                    .max(LayoutUnit::ZERO)
            };
            if let Some(min) = min_size {
                sz = sz.max(min);
            }
            if let Some(max) = max_size {
                sz = sz.min(max);
            }

            // Distribute free space among auto margins if applicable
            let free_space = cb_size
                .saturating_sub(start_val)
                .saturating_sub(end_val)
                .saturating_sub(sz)
                .saturating_sub(non_content);

            if is_margin_start_auto && is_margin_end_auto {
                if free_space >= LayoutUnit::ZERO {
                    margin_start = LayoutUnit::from_raw(free_space.raw() / 2);
                    margin_end = free_space.saturating_sub(margin_start);
                } else {
                    margin_start = LayoutUnit::ZERO;
                    margin_end = free_space;
                }
            } else if is_margin_start_auto {
                margin_start = free_space.saturating_sub(margin_end);
            } else if is_margin_end_auto {
                margin_end = free_space.saturating_sub(margin_start);
            }

            (start_val, sz)
        }
        // Case 3: start, end, and size are all specified
        (Some(start_val), Some(end_val), Some(sz_val)) => {
            let mut sz = sz_val;
            if let Some(min) = min_size {
                sz = sz.max(min);
            }
            if let Some(max) = max_size {
                sz = sz.min(max);
            }

            let free_space = cb_size
                .saturating_sub(start_val)
                .saturating_sub(end_val)
                .saturating_sub(sz)
                .saturating_sub(non_content);

            if is_margin_start_auto && is_margin_end_auto {
                if free_space >= LayoutUnit::ZERO {
                    margin_start = LayoutUnit::from_raw(free_space.raw() / 2);
                    margin_end = free_space.saturating_sub(margin_start);
                } else {
                    margin_start = LayoutUnit::ZERO;
                    margin_end = free_space;
                }
            } else if is_margin_start_auto {
                margin_start = free_space.saturating_sub(margin_end);
            } else if is_margin_end_auto {
                margin_end = free_space.saturating_sub(margin_start);
            }

            (start_val, sz)
        }
        // Case 4: start is specified, end is auto
        (Some(start_val), None, size_val_opt) => {
            let mut sz = size_val_opt.unwrap_or(intrinsic_size);
            if let Some(min) = min_size {
                sz = sz.max(min);
            }
            if let Some(max) = max_size {
                sz = sz.min(max);
            }
            (start_val, sz)
        }
        // Case 5: end is specified, start is auto
        (None, Some(end_val), size_val_opt) => {
            let mut sz = size_val_opt.unwrap_or(intrinsic_size);
            if let Some(min) = min_size {
                sz = sz.max(min);
            }
            if let Some(max) = max_size {
                sz = sz.min(max);
            }
            let start_val = cb_size
                .saturating_sub(end_val)
                .saturating_sub(sz)
                .saturating_sub(non_content)
                .saturating_sub(margin_start)
                .saturating_sub(margin_end);
            (start_val, sz)
        }
        // Case 6: only size is specified, start and end are auto
        (None, None, Some(sz_val)) => {
            let mut sz = sz_val;
            if let Some(min) = min_size {
                sz = sz.max(min);
            }
            if let Some(max) = max_size {
                sz = sz.min(max);
            }
            (LayoutUnit::ZERO, sz)
        }
    };

    (start, content_size, margin_start, margin_end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ace_style::computed::ComputedStyle;
    use ace_style::values::enums::{Display, Position};

    fn make_style_with_insets(
        top: Option<i32>,
        right: Option<i32>,
        bottom: Option<i32>,
        left: Option<i32>,
        width: Option<i32>,
        height: Option<i32>,
        auto_margins: bool,
    ) -> std::sync::Arc<ComputedStyle> {
        let mut style = ComputedStyle::initial();
        let mut b = (*style.box_style).clone();
        b.display = Display::Block;
        b.position = Position::Absolute;
        b.top = top.map(LayoutUnit::from_px);
        b.right = right.map(LayoutUnit::from_px);
        b.bottom = bottom.map(LayoutUnit::from_px);
        b.left = left.map(LayoutUnit::from_px);
        b.width = width.map(LayoutUnit::from_px);
        b.height = height.map(LayoutUnit::from_px);
        style.box_style = std::sync::Arc::new(b);

        if auto_margins {
            let mut s = (*style.surrounding).clone();
            s.margin.left = None;
            s.margin.right = None;
            s.margin.top = None;
            s.margin.bottom = None;
            style.surrounding = std::sync::Arc::new(s);
        }

        std::sync::Arc::new(style)
    }

    #[test]
    fn test_absolute_explicit_insets_and_size() {
        let cb = ContainingBlock::from_viewport(LogicalSize::new(
            LayoutUnit::from_px(800),
            LayoutUnit::from_px(600),
        ));
        let style = make_style_with_insets(
            Some(20),
            Some(50),
            None,
            Some(30),
            Some(200),
            Some(100),
            false,
        );

        let res = resolve_absolute_position(
            &style,
            &cb,
            LayoutUnit::from_px(50),
            LayoutUnit::from_px(50),
        );

        assert_eq!(res.border_box.inline_start, LayoutUnit::from_px(30));
        assert_eq!(res.border_box.block_start, LayoutUnit::from_px(20));
        assert_eq!(res.border_box.inline_size, LayoutUnit::from_px(200));
        assert_eq!(res.border_box.block_size, LayoutUnit::from_px(100));
    }

    #[test]
    fn test_absolute_auto_margin_centering() {
        let cb = ContainingBlock::from_viewport(LogicalSize::new(
            LayoutUnit::from_px(800),
            LayoutUnit::from_px(600),
        ));
        // left: 0, right: 0, width: 200, margin: auto -> centered: (800 - 200) / 2 = 300px
        let style = make_style_with_insets(
            Some(0),
            Some(0),
            Some(0),
            Some(0),
            Some(200),
            Some(100),
            true,
        );

        let res = resolve_absolute_position(
            &style,
            &cb,
            LayoutUnit::from_px(50),
            LayoutUnit::from_px(50),
        );

        // Center horizontally: 300px margin
        assert_eq!(res.margins.inline_start, LayoutUnit::from_px(300));
        assert_eq!(res.margins.inline_end, LayoutUnit::from_px(300));
        assert_eq!(res.border_box.inline_start, LayoutUnit::from_px(300));

        // Center vertically: (600 - 100) / 2 = 250px margin
        assert_eq!(res.margins.block_start, LayoutUnit::from_px(250));
        assert_eq!(res.margins.block_end, LayoutUnit::from_px(250));
        assert_eq!(res.border_box.block_start, LayoutUnit::from_px(250));
    }

    #[test]
    fn test_absolute_stretch_auto_width_between_insets() {
        let cb = ContainingBlock::from_viewport(LogicalSize::new(
            LayoutUnit::from_px(500),
            LayoutUnit::from_px(400),
        ));
        // left: 50, right: 100, width: auto -> width = 500 - 50 - 100 = 350px
        let style =
            make_style_with_insets(None, Some(100), None, Some(50), None, None, false);

        let res = resolve_absolute_position(
            &style,
            &cb,
            LayoutUnit::from_px(50),
            LayoutUnit::from_px(50),
        );

        assert_eq!(res.border_box.inline_start, LayoutUnit::from_px(50));
        assert_eq!(res.border_box.inline_size, LayoutUnit::from_px(350));
    }
}
