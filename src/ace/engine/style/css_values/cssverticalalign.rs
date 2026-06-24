use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssVerticalAlign {
    Baseline,
    Sub,
    Super,
    Top,
    TextTop,
    Middle,
    Bottom,
    TextBottom,
    Length(CssLength),
}

impl Default for CssVerticalAlign {
pub(crate) fn default() -> Self {
        CssVerticalAlign::Baseline
    }
}
