#[derive(Debug, Clone)]
pub struct FloatRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl FloatRect {
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }
    pub fn right(&self) -> f32 {
        self.x + self.width
    }
}

#[derive(Default, Debug, Clone)]
pub struct FloatContext {
    pub left_floats: Vec<FloatRect>,
    pub right_floats: Vec<FloatRect>,
}

impl FloatContext {
    pub fn get_left_offset(&self, y_min: f32, y_max: f32) -> f32 {
        let mut offset = 0.0_f32;
        for f in &self.left_floats {
            if y_min < f.bottom() && y_max > f.y {
                if f.right() > offset {
                    offset = f.right();
                }
            }
        }
        offset
    }

    pub fn get_right_offset(&self, y_min: f32, y_max: f32, container_width: f32) -> f32 {
        let mut limit = container_width;
        for f in &self.right_floats {
            if y_min < f.bottom() && y_max > f.y {
                if f.x < limit {
                    limit = f.x;
                }
            }
        }
        limit
    }

    pub fn get_clear_y(
        &self,
        clear_type: &crate::ace::engine::style::css_values::CssClear,
        current_y: f32,
    ) -> f32 {
        use crate::ace::engine::style::css_values::CssClear;
        let mut new_y = current_y;

        let check_left = matches!(clear_type, CssClear::Left | CssClear::Both);
        let check_right = matches!(clear_type, CssClear::Right | CssClear::Both);

        if check_left {
            for f in &self.left_floats {
                if f.bottom() > new_y {
                    new_y = f.bottom();
                }
            }
        }
        if check_right {
            for f in &self.right_floats {
                if f.bottom() > new_y {
                    new_y = f.bottom();
                }
            }
        }

        new_y
    }
}

#[derive(Clone, Debug)]
pub struct DisplayItem {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub background_color: Option<tiny_skia::Color>,
    pub border_width: f32,
    pub border_color: Option<tiny_skia::Color>,
    pub text_content: Option<std::sync::Arc<str>>,
    pub text_color: tiny_skia::Color,
    pub text_overflow: crate::ace::engine::style::css_values::CssTextOverflow,
    pub font_size: f32,
    pub image_url: Option<std::sync::Arc<str>>,
    pub link_url: Option<std::sync::Arc<str>>,
    pub node_idx: usize,
    pub element_type: crate::ace::engine::types::ElementRenderType,
    pub is_fixed: bool,
    pub opacity: f32,
    pub border_radius: [f32; 4],
    pub transform_rotate: f32,
    pub transform_scale: (f32, f32),
    pub transform_translate: (f32, f32),
    pub canvas_data: Option<Vec<u8>>,

    // Spacing
    pub letter_spacing: f32,
    pub word_spacing: f32,

    // Form Extensions
    pub input_value: std::sync::Arc<str>,
    pub placeholder: std::sync::Arc<str>,
    pub input_type: crate::ace::engine::types::FormInputType,
    pub input_min: std::sync::Arc<str>,
    pub input_max: std::sync::Arc<str>,
    pub input_step: std::sync::Arc<str>,
    pub options: std::sync::Arc<str>,

    // Padding for box model rendering
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,

    // CSS text color mapped earlier
    // CSS font-weight ("normal", "bold", "100"-"900")
    pub font_weight: crate::ace::engine::style::css_values::CssFontWeight,
    // CSS white-space ("normal", "nowrap", "pre", etc.)
    pub white_space: crate::ace::engine::style::css_values::CssWhiteSpace,
    // Border style ("solid", "dashed", "dotted", "none", etc.)
    pub border_style: crate::ace::engine::types::BorderStyle,
    pub is_hovered: bool,
    pub is_focused: bool,

    // Clipping viewport (for overflow: hidden/scroll/auto)
    pub clip_rect: Option<[f32; 4]>,
}

