use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssDisplay {
    None,
    Block,
    Inline,
    InlineBlock,
    Flex,
    InlineFlex,
    Grid,
    Contents,
    Table,
    TableRow,
    TableCell,
    TableHeader,
}

impl Default for CssDisplay {
pub(crate) fn default() -> Self {
        Self::Inline
    }
}
