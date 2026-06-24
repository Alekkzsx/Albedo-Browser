use super::*;
use std::fmt;


#[derive(Debug, Clone, PartialEq)]
pub enum CssBoxSizing {
    ContentBox,
    BorderBox,
}

impl Default for CssBoxSizing {
pub(crate) fn default() -> Self {
        Self::ContentBox
    }
}
