use super::*;
use std::fmt;



/// CSS content property for pseudo-elements (::before, ::after)
#[derive(Debug, Clone, PartialEq)]
pub enum CssContent {
    None,
    Normal,
    String(String),
    // Future: Url, Counter, Attr, etc.
}

impl Default for CssContent {
pub(crate) fn default() -> Self {
        Self::Normal
    }
}
