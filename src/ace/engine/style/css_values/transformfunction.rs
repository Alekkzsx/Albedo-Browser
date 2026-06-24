use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum TransformFunction {
    Translate(CssLength, CssLength),
    TranslateX(CssLength),
    TranslateY(CssLength),
    Scale(f32, f32),
    Rotate(f32), // degrees
    RotateX(f32),
    RotateY(f32),
    RotateZ(f32),
    Skew(f32, f32),
}
