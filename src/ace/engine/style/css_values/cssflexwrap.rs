use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssFlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

impl Default for CssFlexWrap {
pub(crate) fn default() -> Self {
        Self::NoWrap
    }
}
