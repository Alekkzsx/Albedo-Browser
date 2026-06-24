use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssTextOverflow {
    Clip,
    Ellipsis,
}

impl Default for CssTextOverflow {
pub(crate) fn default() -> Self {
        Self::Clip
    }
}
