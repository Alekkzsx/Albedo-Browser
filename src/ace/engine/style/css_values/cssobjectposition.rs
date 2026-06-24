use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub struct CssObjectPosition {
    pub x: CssLength,
    pub y: CssLength,
}

impl Default for CssObjectPosition {
pub(crate) fn default() -> Self {
        Self {
            x: CssLength::Percent(50.0),
            y: CssLength::Percent(50.0),
        }
    }
}
