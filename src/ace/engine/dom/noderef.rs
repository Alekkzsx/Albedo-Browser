use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeRef {
    id: usize,
}

impl NodeRef {
    /// TODO: add docs
    pub fn id(self) -> usize {
        self.id
    }
}
