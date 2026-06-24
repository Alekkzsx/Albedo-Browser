use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssFilter {
    Blur(CssLength),
    Brightness(f32),
    Contrast(f32),
    Grayscale(f32),
    HueRotate(f32), // degrees
    Invert(f32),
    Opacity(f32),
    Saturate(f32),
    Sepia(f32),
    DropShadow(BoxShadow),
}
