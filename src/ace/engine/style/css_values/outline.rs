use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub struct Outline {
    pub width: f32,
    pub color: CssColor,
    pub style: String, // "solid", "dashed", "dotted", etc.
    pub offset: f32,
}
