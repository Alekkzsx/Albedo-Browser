use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssVisibility {
    Visible,
    Hidden,
    Collapse,
}

impl Default for CssVisibility {
pub(crate) fn default() -> Self {
        Self::Visible
    }
}
