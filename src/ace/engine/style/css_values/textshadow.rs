use super::*;
use std::fmt;



/// Text shadow structure for text-shadow CSS property
#[derive(Debug, Clone, PartialEq)]
pub struct TextShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub color: CssColor,
}

impl Default for TextShadow {
pub(crate) fn default() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            blur: 0.0,
            color: CssColor::Named("black".to_string()),
        }
    }
}
