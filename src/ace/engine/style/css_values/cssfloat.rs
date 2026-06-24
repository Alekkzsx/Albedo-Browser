use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssFloat {
    None,
    Left,
    Right,
}

impl Default for CssFloat {
pub(crate) fn default() -> Self {
        Self::None
    }
}
