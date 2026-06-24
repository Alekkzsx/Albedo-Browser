use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssObjectFit {
    Fill,
    Contain,
    Cover,
    None,
    ScaleDown,
}

impl Default for CssObjectFit {
pub(crate) fn default() -> Self {
        Self::Fill
    }
}
