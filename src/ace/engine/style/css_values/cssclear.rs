use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssClear {
    None,
    Left,
    Right,
    Both,
}

impl Default for CssClear {
pub(crate) fn default() -> Self {
        Self::None
    }
}
