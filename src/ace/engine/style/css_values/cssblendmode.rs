use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssBlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl Default for CssBlendMode {
pub(crate) fn default() -> Self {
        Self::Normal
    }
}
