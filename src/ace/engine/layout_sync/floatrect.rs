use super::*;

use crate::ace::engine::dom::AceDOM;
use crate::ace::engine::layout::ElementGeometry;
use crate::ace::engine::core::AceEngine;



#[derive(Debug, Clone)]
pub struct FloatRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl FloatRect {
    /// TODO: add docs
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }
    /// TODO: add docs
    pub fn right(&self) -> f32 {
        self.x + self.width
    }
}
