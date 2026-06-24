use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssJustifyContent {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl Default for CssJustifyContent {
pub(crate) fn default() -> Self {
        Self::FlexStart
    }
}
