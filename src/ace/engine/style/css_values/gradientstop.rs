use super::*;
use std::fmt;



/// Gradient color stop
#[derive(Debug, Clone, PartialEq)]
pub struct GradientStop {
    pub color: CssColor,
    pub position: Option<f32>, // 0.0 to 1.0
}
