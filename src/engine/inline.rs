/// Inline Formatting Context (IFC) - Algorithm for laying out inline elements and text
/// Based on CSS 2.2 / CSS 3 specification for inline layout

use crate::engine::style::css_values::{ComputedStyle, CssDisplay, CssVerticalAlign};
use crate::engine::text::TextMetrics;

/// Represents an inline box (can be text, inline element, or inline-block)
#[derive(Clone, Debug)]
pub enum InlineBox {
    /// Text content with metrics
    Text {
        content: String,
        metrics: TextMetrics,
        style: ComputedStyle,
    },
    /// Inline element (span, strong, em, etc.)
    Inline {
        children: Vec<InlineBox>,
        style: ComputedStyle,
    },
    /// Inline-block element (block content displayed inline)
    InlineBlock {
        width: f32,
        height: f32,
        style: ComputedStyle,
    },
    /// Hard break (explicit newline in white-space: pre)
    HardBreak,
}

impl InlineBox {
    /// Get the inline size (width) of this box
    pub fn inline_size(&self) -> f32 {
        match self {
            InlineBox::Text { metrics, .. } => metrics.width,
            InlineBox::Inline { .. } => 0.0, // Container, width depends on children
            InlineBox::InlineBlock { width, .. } => *width,
            InlineBox::HardBreak => 0.0,
        }
    }

    /// Get ascent (distance from baseline to top)
    pub fn ascent(&self) -> f32 {
        match self {
            InlineBox::Text { metrics, .. } => metrics.ascent,
            InlineBox::InlineBlock { height, .. } => *height * 0.8, // Assume 80/20 split
            _ => 0.0,
        }
    }

    /// Get descent (distance from baseline to bottom)
    pub fn descent(&self) -> f32 {
        match self {
            InlineBox::Text { metrics, .. } => metrics.descent,
            InlineBox::InlineBlock { height, .. } => *height * 0.2,
            _ => 0.0,
        }
    }

    /// Get total height
    pub fn height(&self) -> f32 {
        self.ascent() + self.descent()
    }
}

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

/// Inline Formatting Context - lays out inline content
pub struct InlineFormattingContext {
    /// Available width for layout
    pub available_width: f32,
    /// Current lines
    pub lines: Vec<LayoutLine>,
    /// Current line being built
    current_line: LayoutLine,
}

impl InlineFormattingContext {
    pub fn new(available_width: f32) -> Self {
        Self {
            available_width,
            lines: Vec::new(),
            current_line: LayoutLine::new(),
        }
    }

    /// Layout a sequence of inline boxes
    pub fn layout(&mut self, boxes: Vec<InlineBox>) {
        for box_ in boxes {
            match box_ {
                InlineBox::HardBreak => {
                    // Force line break
                    self.finish_line();
                }
                InlineBox::Text { .. } => {
                    let box_width = box_.inline_size();

                    // Check if box fits on current line
                    if self.current_line.width + box_width > self.available_width && !self.current_line.boxes.is_empty() {
                        // Box doesn't fit, start new line
                        self.finish_line();
                    }

                    self.current_line.add_box(box_);
                }
                InlineBox::Inline { children, .. } => {
                    // Recursively layout inline element's children
                    self.layout(children);
                }
                InlineBox::InlineBlock { .. } => {
                    let box_width = box_.inline_size();

                    // Check if box fits on current line
                    if self.current_line.width + box_width > self.available_width && !self.current_line.boxes.is_empty() {
                        // Box doesn't fit, start new line
                        self.finish_line();
                    }

                    self.current_line.add_box(box_);
                }
            }
        }

        // Finish final line
        if !self.current_line.boxes.is_empty() {
            self.finish_line();
        }
    }

    /// Finish current line and start a new one
    fn finish_line(&mut self) {
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
        self.lines.iter()
            .map(|line| line.width)
            .fold(0.0, f32::max)
    }
}

/// Word-breaking and line-breaking algorithm
pub struct LineBreaker;

impl LineBreaker {
    /// Break text into lines based on width constraint
    /// Returns vector of (text, width) tuples
    pub fn break_text(text: &str, max_width: f32, measurer: &crate::engine::text::TextMeasurer, font_size: f32, line_height: f32, family: Option<&str>, weight: cosmic_text::Weight) -> Vec<(String, f32)> {
        // Use cosmic-text's built-in wrapping
        measurer.measure_text_wrapped(text, font_size, line_height, family, weight, max_width, None)
    }

    /// Break text at word boundaries
    pub fn break_words(text: &str) -> Vec<&str> {
        text.split_whitespace().collect()
    }

    /// Break text at character boundaries (for break-all)
    pub fn break_all(text: &str, _max_width: f32) -> Vec<String> {
        text.chars()
            .collect::<Vec<_>>()
            .iter()
            .map(|c| c.to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inline_box_dimensions() {
        let metrics = TextMetrics {
            width: 100.0,
            height: 20.0,
            ascent: 16.0,
            descent: 4.0,
            line_height: 20.0,
        };

        let style = ComputedStyle::default();
        let box_ = InlineBox::Text {
            content: "test".to_string(),
            metrics,
            style,
        };

        assert_eq!(box_.inline_size(), 100.0);
        assert_eq!(box_.ascent(), 16.0);
        assert_eq!(box_.descent(), 4.0);
        assert_eq!(box_.height(), 20.0);
    }

    #[test]
    fn test_layout_line() {
        let metrics = TextMetrics {
            width: 50.0,
            height: 20.0,
            ascent: 16.0,
            descent: 4.0,
            line_height: 20.0,
        };

        let style = ComputedStyle::default();
        let mut line = LayoutLine::new();

        let box1 = InlineBox::Text {
            content: "hello".to_string(),
            metrics,
            style: style.clone(),
        };

        let box2 = InlineBox::Text {
            content: "world".to_string(),
            metrics,
            style,
        };

        line.add_box(box1);
        line.add_box(box2);

        assert_eq!(line.width, 100.0);
        assert_eq!(line.height, 20.0);
        assert_eq!(line.boxes.len(), 2);
    }

    #[test]
    fn test_inline_formatting_context() {
        let metrics = TextMetrics {
            width: 50.0,
            height: 20.0,
            ascent: 16.0,
            descent: 4.0,
            line_height: 20.0,
        };

        let style = ComputedStyle::default();
        let mut ifc = InlineFormattingContext::new(120.0);

        let boxes = vec![
            InlineBox::Text {
                content: "hello".to_string(),
                metrics,
                style: style.clone(),
            },
            InlineBox::Text {
                content: "world".to_string(),
                metrics,
                style,
            },
        ];

        ifc.layout(boxes);

        // Both boxes should fit on one line (50 + 50 = 100 < 120)
        assert_eq!(ifc.lines.len(), 1);
        assert_eq!(ifc.lines[0].boxes.len(), 2);
    }

    #[test]
    fn test_inline_formatting_context_wrapping() {
        let metrics = TextMetrics {
            width: 80.0,
            height: 20.0,
            ascent: 16.0,
            descent: 4.0,
            line_height: 20.0,
        };

        let style = ComputedStyle::default();
        let mut ifc = InlineFormattingContext::new(120.0);

        let boxes = vec![
            InlineBox::Text {
                content: "hello".to_string(),
                metrics,
                style: style.clone(),
            },
            InlineBox::Text {
                content: "world".to_string(),
                metrics,
                style,
            },
        ];

        ifc.layout(boxes);

        // First box (80) fits on line 1
        // Second box (80) doesn't fit (80 + 80 = 160 > 120), goes to line 2
        assert_eq!(ifc.lines.len(), 2);
        assert_eq!(ifc.lines[0].boxes.len(), 1);
        assert_eq!(ifc.lines[1].boxes.len(), 1);
    }
}
