use super::*;
/// Inline Formatting Context (IFC) - Algorithm for laying out inline elements and text
/// Based on CSS 2.2 / CSS 3 specification for inline layout
use crate::ace::engine::style::css_values::ComputedStyle;
use crate::ace::engine::text::TextMetrics;

/// Represents an inline box (can be text, inline element, or inline-block)


/// Represents a single line in the inline formatting context
#[derive(Clone, Debug)]
pub struct LayoutLine {
    /// Boxes on this line
    pub boxes: Vec<(InlineBox, f32, f32)>, // (box, x_offset, y_offset)
    /// Width of the line
    pub width: f32,
    /// Height of the line (from baseline)
    pub height: f32,
    /// Baseline position relative to top of line
    pub baseline: f32,
}

impl LayoutLine {
    /// TODO: add docs
    pub fn new() -> Self {
        Self {
            boxes: Vec::new(),
            width: 0.0,
            height: 0.0,
            baseline: 0.0,
        }
    }

    /// Add an inline box to this line
    pub fn add_box(&mut self, box_: InlineBox) {
        let box_width = box_.inline_size();
        let box_ascent = box_.ascent();
        let box_descent = box_.descent();

        // Update line baseline and height (highest ascent + highest descent)
        self.baseline = self.baseline.max(box_ascent);
        self.height = self.height.max(box_ascent + box_descent);

        // Add box at current line width
        self.boxes.push((box_, self.width, 0.0));
        self.width += box_width;
    }

    /// Align all boxes on this line vertically based on vertical-align
    pub fn finalize_alignment(&mut self) {
        // All boxes should be positioned relative to baseline
        for (_, _, y_offset) in &mut self.boxes {
            *y_offset = self.baseline;
        }
    }
}
