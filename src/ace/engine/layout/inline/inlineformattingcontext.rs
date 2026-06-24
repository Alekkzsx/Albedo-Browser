use super::*;
/// Inline Formatting Context (IFC) - Algorithm for laying out inline elements and text
/// Based on CSS 2.2 / CSS 3 specification for inline layout
use crate::ace::engine::style::css_values::ComputedStyle;
use crate::ace::engine::text::TextMetrics;

/// Represents an inline box (can be text, inline element, or inline-block)


/// Inline Formatting Context - lays out inline content
pub struct InlineFormattingContext {
    /// Available width for layout
    pub available_width: f32,
    /// Current lines
    pub lines: Vec<LayoutLine>,
    /// Current line being built
    current_line: LayoutLine,
    /// Text measurer for ellipsis calculations
    pub text_measurer: crate::ace::engine::text::TextMeasurer,
    /// Text overflow policy
    pub text_overflow: crate::ace::engine::style::css_values::CssTextOverflow,
}
