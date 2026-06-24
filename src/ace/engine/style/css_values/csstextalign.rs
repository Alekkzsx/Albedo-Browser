use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssTextAlign {
    Left,
    Right,
    Center,
    Justify,
    Start,
    End,
}

impl Default for CssTextAlign {
pub(crate) fn default() -> Self {
        Self::Left
    }
}
