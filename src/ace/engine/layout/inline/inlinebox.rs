use super::*;
/// Inline Formatting Context (IFC) - Algorithm for laying out inline elements and text
/// Based on CSS 2.2 / CSS 3 specification for inline layout
use crate::ace::engine::style::css_values::ComputedStyle;
use crate::ace::engine::text::TextMetrics;

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
