use super::*;
use std::fmt;



/// Background image - supports colors, gradients, and URLs
#[derive(Debug, Clone, PartialEq)]
pub enum BackgroundImage {
    None,
    Color(CssColor),
    Gradient(Gradient),
    Url(String),
}

impl Default for BackgroundImage {
pub(crate) fn default() -> Self {
        Self::None
    }
}
