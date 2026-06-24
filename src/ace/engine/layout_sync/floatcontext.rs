use super::*;

use crate::ace::engine::dom::AceDOM;
use crate::ace::engine::layout::ElementGeometry;
use crate::ace::engine::core::AceEngine;




#[derive(Default, Debug, Clone)]
pub struct FloatContext {
    pub left_floats: Vec<FloatRect>,
    pub right_floats: Vec<FloatRect>,
}

impl FloatContext {
    /// TODO: add docs
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

    /// TODO: add docs
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

    /// TODO: add docs
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
