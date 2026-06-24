use super::*;
/// Inline Formatting Context (IFC) - Algorithm for laying out inline elements and text
/// Based on CSS 2.2 / CSS 3 specification for inline layout
use crate::ace::engine::style::css_values::ComputedStyle;
use crate::ace::engine::text::TextMetrics;

/// Represents an inline box (can be text, inline element, or inline-block)


impl InlineFormattingContext {

    /// Finish current line and start a new one
pub(crate) fn finish_line(&mut self) {
        if !self.current_line.boxes.is_empty() {
            self.current_line.finalize_alignment();
            self.lines.push(self.current_line.clone());
            self.current_line = LayoutLine::new();
        }
    }

    /// Get total height of all lines
    pub fn total_height(&self) -> f32 {
        self.lines.iter().map(|line| line.height).sum()
    }

    /// Get maximum width used
    pub fn max_width(&self) -> f32 {
        self.lines.iter().map(|line| line.width).fold(0.0, f32::max)
    }
}
