use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssPosition {
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

impl Default for CssPosition {
pub(crate) fn default() -> Self {
        Self::Static
    }
}
