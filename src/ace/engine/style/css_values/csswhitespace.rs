use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssWhiteSpace {
    Normal,
    NoWrap,
    Pre,
    PreWrap,
    PreLine,
}

impl Default for CssWhiteSpace {
pub(crate) fn default() -> Self {
        Self::Normal
    }
}
