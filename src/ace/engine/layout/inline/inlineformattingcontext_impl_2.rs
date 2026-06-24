use super::*;
/// Inline Formatting Context (IFC) - Algorithm for laying out inline elements and text
/// Based on CSS 2.2 / CSS 3 specification for inline layout
use crate::ace::engine::style::css_values::ComputedStyle;
use crate::ace::engine::text::TextMetrics;

/// Represents an inline box (can be text, inline element, or inline-block)


impl InlineFormattingContext {

    /// Apply overflow logic (ellipsis or clip) to a text box
pub(crate) fn apply_overflow_logic(&mut self, box_: InlineBox, use_ellipsis: bool) {
        if let InlineBox::Text {
            content,
            metrics,
            style,
        } = box_
        {
            let font_size = style.font_size;
            let line_height = crate::ace::engine::style::css_values::resolve_length(
                &style.line_height,
                font_size,
                16.0,
                self.available_width,
                0.0,
            ); // Simple resolve
            let line_height = if line_height <= 0.0 {
                font_size * 1.2
            } else {
                line_height
            };

            let letter_spacing = crate::ace::engine::style::css_values::resolve_length(
                &style.letter_spacing,
                font_size,
                16.0,
                self.available_width,
                0.0,
            );
            let word_spacing = crate::ace::engine::style::css_values::resolve_length(
                &style.word_spacing,
                font_size,
                16.0,
                self.available_width,
                0.0,
            );

            let ellipsis = "…";
            let (ell_w, _) = if use_ellipsis {
                self.text_measurer.measure_text(
                    ellipsis,
                    font_size,
                    line_height,
                    Some(&style.font_family),
                    cosmic_text::Weight::NORMAL,
                    None,
                    letter_spacing,
                    word_spacing,
                )
            } else {
                (0.0, 0.0)
            };

            let available = self.available_width - self.current_line.width;
            let safe_width = available - ell_w;

            if safe_width <= 0.0 {
                if use_ellipsis {
                    let ell_metrics = crate::ace::engine::text::TextMetrics {
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
                    while mid_adj > 0 && !content.is_char_boundary(mid_adj) {
                        mid_adj -= 1;
                    }

                    let sub = &content[..mid_adj];
                    let (w, _) = self.text_measurer.measure_text(
                        sub,
                        font_size,
                        line_height,
                        Some(&style.font_family),
                        cosmic_text::Weight::NORMAL,
                        None,
                        letter_spacing,
                        word_spacing,
                    );

                    if w <= safe_width {
                        best_idx = mid_adj;
                        left = mid + 1;
                        while left < content.len() && !content.is_char_boundary(left) {
                            left += 1;
                        }
                    } else {
                        right = mid.saturating_sub(1);
                    }
                }

                let mut result_content = content[..best_idx].to_string();
                if use_ellipsis {
                    result_content.push_str(ellipsis);
                }

                let (new_w, _) = self.text_measurer.measure_text(
                    &result_content,
                    font_size,
                    line_height,
                    Some(&style.font_family),
                    cosmic_text::Weight::NORMAL,
                    None,
                    letter_spacing,
                    word_spacing,
                );

                let new_metrics = crate::ace::engine::text::TextMetrics {
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
}
