use super::*;
/// Inline Formatting Context (IFC) - Algorithm for laying out inline elements and text
/// Based on CSS 2.2 / CSS 3 specification for inline layout
use crate::ace::engine::style::css_values::ComputedStyle;
use crate::ace::engine::text::TextMetrics;

/// Represents an inline box (can be text, inline element, or inline-block)


/// TODO: add docs
pub fn apply_text_transform(
    text: &str,
    transform: &crate::ace::engine::style::css_values::CssTextTransform,
) -> String {
    match transform {
        crate::ace::engine::style::css_values::CssTextTransform::Uppercase => text.to_uppercase(),
        crate::ace::engine::style::css_values::CssTextTransform::Lowercase => text.to_lowercase(),
        crate::ace::engine::style::css_values::CssTextTransform::Capitalize => {
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
        }
        crate::ace::engine::style::css_values::CssTextTransform::None => text.to_string(),
    }
}
