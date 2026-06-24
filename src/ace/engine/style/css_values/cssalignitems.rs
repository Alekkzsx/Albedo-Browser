use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssAlignItems {
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
    Auto, // For align-self
}

impl Default for CssAlignItems {
pub(crate) fn default() -> Self {
        Self::Stretch
    }
}
