use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssAlignContent {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
    Stretch,
}

impl Default for CssAlignContent {
pub(crate) fn default() -> Self {
        Self::Stretch
    }
}
