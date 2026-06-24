use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssCursor {
    Auto,
    Default,
    Pointer,
    Text,
    Wait,
    Help,
    NotAllowed,
    Grab,
    Grabbing,
}

impl Default for CssCursor {
pub(crate) fn default() -> Self {
        Self::Auto
    }
}
