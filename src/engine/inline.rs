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
    /// Text measurer for ellipsis calculations
    pub text_measurer: crate::engine::text::TextMeasurer,
    /// Text overflow policy
    pub text_overflow: crate::engine::style::css_values::CssTextOverflow,
}

impl InlineFormattingContext {
    pub fn new(available_width: f32, text_measurer: crate::engine::text::TextMeasurer, text_overflow: crate::engine::style::css_values::CssTextOverflow) -> Self {
        Self {
            available_width,
            lines: Vec::new(),
            current_line: LayoutLine::new(),
            text_measurer,
            text_overflow,
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
                InlineBox::Text { ref content, ref metrics, ref style } => {
                    let box_width = box_.inline_size();

                    // Check if box fits on current line
                    if self.current_line.width + box_width > self.available_width && !self.current_line.boxes.is_empty() {
                        match self.text_overflow {
                            crate::engine::style::css_values::CssTextOverflow::Ellipsis => {
                                // Handle ellipsis truncation
                                self.apply_overflow_logic(box_.clone(), true);
                                return;
                            }
                            crate::engine::style::css_values::CssTextOverflow::Clip => {
                                // For clip with nowrap, we just add it and it will be physically clipped by the renderer or container
                                // But if it's NOT the first box, we might want to start a new line if not nowrap.
                                // Specification: text-overflow only affects line boxes that overflow in the inline-progression direction.
                                self.finish_line();
                            }
                        }
                    }

                    self.current_line.add_box(box_);
                    
                    // After adding, if it overflows and we have ellipsis/clip on a single line (nowrap logic simulation)
                    if self.current_line.width > self.available_width {
                        match self.text_overflow {
                            crate::engine::style::css_values::CssTextOverflow::Ellipsis => {
                                let last_box = self.current_line.boxes.pop();
                                if let Some((b, _, _)) = last_box {
                                    self.current_line.width -= b.inline_size();
                                    self.apply_overflow_logic(b, true);
                                }
                                return;
                            }
                            crate::engine::style::css_values::CssTextOverflow::Clip => {
                                // Just keep it, renderer handles physical clipping if overflow: hidden is set on parent
                            }
                        }
                    }
                }
                InlineBox::Inline { children, .. } => {
                    // Recursively layout inline element's children
                    self.layout(children);
                }
                InlineBox::InlineBlock { .. } => {
                    let box_width = box_.inline_size();

                    // Check if box fits on current line
                    if self.current_line.width + box_width > self.available_width && !self.current_line.boxes.is_empty() {
                        if !matches!(self.text_overflow, crate::engine::style::css_values::CssTextOverflow::Clip) {
                            // Non-clip (e.g. ellipsis) usually stops here for inline-blocks too if they overflow
                            return;
                        }
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

    /// Apply overflow logic (ellipsis or clip) to a text box
    fn apply_overflow_logic(&mut self, box_: InlineBox, use_ellipsis: bool) {
        if let InlineBox::Text { content, metrics, style } = box_ {
            let font_size = style.font_size;
            let line_height = crate::engine::style::css_values::resolve_length(&style.line_height, font_size, 16.0, self.available_width, 0.0); // Simple resolve
            let line_height = if line_height <= 0.0 { font_size * 1.2 } else { line_height };
            
            let letter_spacing = crate::engine::style::css_values::resolve_length(&style.letter_spacing, font_size, 16.0, self.available_width, 0.0);
            let word_spacing = crate::engine::style::css_values::resolve_length(&style.word_spacing, font_size, 16.0, self.available_width, 0.0);
            
            let ellipsis = "…";
            let (ell_w, _) = if use_ellipsis {
                self.text_measurer.measure_text(ellipsis, font_size, line_height, Some(&style.font_family), cosmic_text::Weight::NORMAL, None, letter_spacing, word_spacing)
            } else {
                (0.0, 0.0)
            };
            
            let available = self.available_width - self.current_line.width;
            let safe_width = available - ell_w;
            
            if safe_width <= 0.0 {
                if use_ellipsis {
                    let ell_metrics = crate::engine::text::TextMetrics {
                        width: ell_w,
                        height: metrics.height,
                        ascent: metrics.ascent,
                        descent: metrics.descent,
                        line_height: metrics.line_height,
                    };
                    self.current_line.add_box(InlineBox::Text {
                        content: ellipsis.to_string(),
                        metrics: ell_metrics,
                        style,
                    });
                }
                // If clip, we don't add anything if there's no space? 
                // Actually, clip means we draw what fits.
            } else {
                // Find truncation point
                let mut left = 0;
                let mut right = content.len();
                let mut best_idx = 0;
                
                while left <= right {
                    let mid = (left + right) / 2;
                    let mut mid_adj = mid;
                    while mid_adj > 0 && !content.is_char_boundary(mid_adj) { mid_adj -= 1; }
                    
                    let sub = &content[..mid_adj];
                    let (w, _) = self.text_measurer.measure_text(sub, font_size, line_height, Some(&style.font_family), cosmic_text::Weight::NORMAL, None, letter_spacing, word_spacing);
                    
                    if w <= safe_width {
                        best_idx = mid_adj;
                        left = mid + 1;
                        while left < content.len() && !content.is_char_boundary(left) { left += 1; }
                    } else {
                        right = mid.saturating_sub(1);
                    }
                }
                
                let mut result_content = content[..best_idx].to_string();
                if use_ellipsis {
                    result_content.push_str(ellipsis);
                }
                
                let (new_w, _) = self.text_measurer.measure_text(&result_content, font_size, line_height, Some(&style.font_family), cosmic_text::Weight::NORMAL, None, letter_spacing, word_spacing);
                
                let new_metrics = crate::engine::text::TextMetrics {
                    width: new_w,
                    height: metrics.height,
                    ascent: metrics.ascent,
                    descent: metrics.descent,
                    line_height: metrics.line_height,
                };
                
                self.current_line.add_box(InlineBox::Text {
                    content: result_content,
                    metrics: new_metrics,
                    style,
                });
            }
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
        // Obsolete
        vec![]
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

pub fn apply_text_transform(text: &str, transform: &crate::engine::style::css_values::CssTextTransform) -> String {
    match transform {
        crate::engine::style::css_values::CssTextTransform::Uppercase => text.to_uppercase(),
        crate::engine::style::css_values::CssTextTransform::Lowercase => text.to_lowercase(),
        crate::engine::style::css_values::CssTextTransform::Capitalize => {
            let mut result = String::with_capacity(text.len());
            let mut capitalize_next = true;
            for c in text.chars() {
                if c.is_whitespace() {
                    capitalize_next = true;
                    result.push(c);
                } else if capitalize_next {
                    result.extend(c.to_uppercase());
                    capitalize_next = false;
                } else {
                    result.push(c);
                }
            }
            result
        },
        crate::engine::style::css_values::CssTextTransform::None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

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
            metrics: metrics.clone(),
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
        let font_system = Arc::new(Mutex::new(cosmic_text::FontSystem::new()));
        let measurer = crate::engine::text::TextMeasurer::new(font_system);
        let mut ifc = InlineFormattingContext::new(120.0, measurer, crate::engine::style::css_values::CssTextOverflow::Clip);

        let boxes = vec![
            InlineBox::Text {
                content: "hello".to_string(),
                metrics: metrics.clone(),
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
        let font_system = Arc::new(Mutex::new(cosmic_text::FontSystem::new()));
        let measurer = crate::engine::text::TextMeasurer::new(font_system);
        let mut ifc = InlineFormattingContext::new(120.0, measurer, crate::engine::style::css_values::CssTextOverflow::Clip);

        let boxes = vec![
            InlineBox::Text {
                content: "hello".to_string(),
                metrics: metrics.clone(),
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

    #[test]
    fn test_inline_formatting_context_ellipsis() {
        let metrics = TextMetrics {
            width: 80.0,
            height: 20.0,
            ascent: 16.0,
            descent: 4.0,
            line_height: 20.0,
        };

        let style = ComputedStyle::default();
        let font_system = Arc::new(Mutex::new(cosmic_text::FontSystem::new()));
        let measurer = crate::engine::text::TextMeasurer::new(font_system);
        let mut ifc = InlineFormattingContext::new(120.0, measurer, crate::engine::style::css_values::CssTextOverflow::Ellipsis);

        let boxes = vec![
            InlineBox::Text {
                content: "Very long text that should be truncated".to_string(),
                metrics,
                style: style.clone(),
            },
        ];

        ifc.layout(boxes);

        assert_eq!(ifc.lines.len(), 1);
        let (box_0, _, _) = &ifc.lines[0].boxes[0];
        if let InlineBox::Text { content, .. } = box_0 {
            assert!(content.contains("…"));
            assert!(ifc.lines[0].width <= 50.1);
        } else {
            panic!("Expected text box");
        }
    }

    #[test]
    fn test_inline_formatting_context_clip() {
        let metrics = TextMetrics {
            width: 80.0,
            height: 20.0,
            ascent: 16.0,
            descent: 4.0,
            line_height: 20.0,
        };

        let style = ComputedStyle::default();
        let font_system = Arc::new(Mutex::new(cosmic_text::FontSystem::new()));
        let measurer = crate::engine::text::TextMeasurer::new(font_system);
        let mut ifc = InlineFormattingContext::new(50.0, measurer, crate::engine::style::css_values::CssTextOverflow::Clip);

        let boxes = vec![
            InlineBox::Text {
                content: "Very long text that should be clipped".to_string(),
                metrics,
                style: style.clone(),
            },
        ];

        ifc.layout(boxes);

        // Clip scenario: we currently allow the box but the line would be marked as overflowing
        assert_eq!(ifc.lines.len(), 1);
        assert_eq!(ifc.lines[0].boxes.len(), 1);
    }
}
