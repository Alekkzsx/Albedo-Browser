use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssTextTransform {
    None,
    Uppercase,
    Lowercase,
    Capitalize,
}

impl Default for CssTextTransform {
pub(crate) fn default() -> Self {
        Self::None
    }
}
