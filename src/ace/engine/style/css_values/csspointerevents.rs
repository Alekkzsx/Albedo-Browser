use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssPointerEvents {
    Auto,
    None,
}

impl Default for CssPointerEvents {
pub(crate) fn default() -> Self {
        Self::Auto
    }
}
