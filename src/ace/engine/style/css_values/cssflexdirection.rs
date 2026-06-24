use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssFlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl Default for CssFlexDirection {
pub(crate) fn default() -> Self {
        Self::Row
    }
}
