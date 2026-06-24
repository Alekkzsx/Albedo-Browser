use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssOverflow {
    Visible,
    Hidden,
    Scroll,
    Auto,
}

impl Default for CssOverflow {
pub(crate) fn default() -> Self {
        Self::Visible
    }
}
