//! Geometry types and managers for layout engine

use crate::ace::engine::style::css_values::{CssFloat, CssClear, CssOverflow};

/// Complete element geometry information including scroll and content dimensions
#[derive(Clone, Debug)]
pub struct ElementGeometry {
    // Layout position and size (from Taffy)
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,

    // Border and padding (from computed style)
    pub border_top: f32,
    pub border_right: f32,
    pub border_bottom: f32,
    pub border_left: f32,
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,

    // Scroll offset for this element
    pub scroll_x: f32,
    pub scroll_y: f32,

    // Content dimensions (min/max bounds of children)
    pub content_width: f32,
    pub content_height: f32,

    // Overflow style
    pub overflow_x: CssOverflow,
    pub overflow_y: CssOverflow,

    // Float & Clear context
    pub float: CssFloat,
    pub clear: CssClear,
}

impl ElementGeometry {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            border_top: 0.0,
            border_right: 0.0,
            border_bottom: 0.0,
            border_left: 0.0,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            content_width: 0.0,
            content_height: 0.0,
            overflow_x: CssOverflow::Visible,
            overflow_y: CssOverflow::Visible,
            float: CssFloat::None,
            clear: CssClear::None,
        }
    }

    /// Client width: content width - padding (no border) - Actually width - borders
    pub fn client_width(&self) -> f32 {
        (self.width - self.border_left - self.border_right).max(0.0)
    }

    /// Client height: content height - padding (no border) - Actually height - borders
    pub fn client_height(&self) -> f32 {
        (self.height - self.border_top - self.border_bottom).max(0.0)
    }

    /// Scroll width: max of client width and content width
    pub fn scroll_width(&self) -> f32 {
        self.content_width.max(self.client_width())
    }

    /// Scroll height: max of client height and content height
    pub fn scroll_height(&self) -> f32 {
        self.content_height.max(self.client_height())
    }
}

impl Default for ElementGeometry {
    fn default() -> Self {
        Self::new()
    }
}

/// Grid layout context
#[derive(Clone, Debug, Default)]
pub struct GridContext {
    pub column_names: std::collections::HashMap<String, Vec<i16>>,
    pub row_names: std::collections::HashMap<String, Vec<i16>>,
    pub areas: std::collections::HashMap<String, (usize, usize, usize, usize)>,
    pub col_offset: i16,
    pub row_offset: i16,
}

/// Primitive ACE element for rendering
#[derive(Clone, Debug)]
pub struct ACEPrimitive {
    pub node_idx: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub style: crate::ace::engine::style::Declaration,
    // Form Extensions
    pub input_value: String,
    pub placeholder: String,
    pub input_type: String,
    pub options: String,
}

/// Manages dirty rects for invalidation
#[derive(Clone, Debug, Default)]
pub struct InvalidationManager {
    pub dirty_rects: Vec<tiny_skia::Rect>,
}

impl InvalidationManager {
    pub fn new() -> Self {
        Self {
            dirty_rects: Vec::new(),
        }
    }

    pub fn add_dirty_rect(&mut self, rect: tiny_skia::Rect) {
        let mut merged = false;
        for existing in &mut self.dirty_rects {
            if let Some(union_rect) = Self::union_rect(*existing, rect) {
                *existing = union_rect;
                merged = true;
                break;
            }
        }
        if !merged {
            self.dirty_rects.push(rect);
        }
    }

    pub fn union_rect(a: tiny_skia::Rect, b: tiny_skia::Rect) -> Option<tiny_skia::Rect> {
        let left = a.left().min(b.left());
        let right = a.right().max(b.right());
        let top = a.top().min(b.top());
        let bottom = a.bottom().max(b.bottom());

        tiny_skia::Rect::from_ltrb(left, top, right, bottom)
    }

    pub fn clear(&mut self) {
        self.dirty_rects.clear();
    }
}
