use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


#[derive(Clone, Debug)]
pub struct MutationRecord {
    pub type_: MutationType,
    pub target: usize,
    pub added_nodes: Vec<usize>,
    pub removed_nodes: Vec<usize>,
    pub previous_sibling: Option<usize>,
    pub next_sibling: Option<usize>,
    pub attribute_name: Option<String>,
    pub old_value: Option<String>,
}
