use super::*;
/// Inline Formatting Context (IFC) - Algorithm for laying out inline elements and text
/// Based on CSS 2.2 / CSS 3 specification for inline layout
use crate::ace::engine::style::css_values::ComputedStyle;
use crate::ace::engine::text::TextMetrics;

/// Represents an inline box (can be text, inline element, or inline-block)


impl InlineFormattingContext {
    /// TODO: add docs
    pub fn new(
        available_width: f32,
        text_measurer: crate::ace::engine::text::TextMeasurer,
        text_overflow: crate::ace::engine::style::css_values::CssTextOverflow,
    ) -> Self {
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
                InlineBox::Text {
                    content: _,
                    metrics: _,
                    style: _,
                } => {
                    let box_width = box_.inline_size();

                    // Check if box fits on current line
                    if self.current_line.width + box_width > self.available_width
                        && !self.current_line.boxes.is_empty()
                    {
                        match self.text_overflow {
                            crate::ace::engine::style::css_values::CssTextOverflow::Ellipsis => {
                                // Handle ellipsis truncation
                                self.apply_overflow_logic(box_.clone(), true);
                                return;
                            }
                            crate::ace::engine::style::css_values::CssTextOverflow::Clip => {
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
                            crate::ace::engine::style::css_values::CssTextOverflow::Ellipsis => {
                                let last_box = self.current_line.boxes.pop();
                                if let Some((b, _, _)) = last_box {
                                    self.current_line.width -= b.inline_size();
                                    self.apply_overflow_logic(b, true);
                                }
                                return;
                            }
                            crate::ace::engine::style::css_values::CssTextOverflow::Clip => {
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
                    if self.current_line.width + box_width > self.available_width
                        && !self.current_line.boxes.is_empty()
                    {
                        if !matches!(
                            self.text_overflow,
                            crate::ace::engine::style::css_values::CssTextOverflow::Clip
                        ) {
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
}
