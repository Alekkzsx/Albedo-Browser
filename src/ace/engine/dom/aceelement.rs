use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


#[derive(Clone, Debug, PartialEq)]
pub struct AceElement {
    pub tag: String,
    pub namespace: crate::ace::html::Namespace,
    pub attributes: HashMap<String, String>,
}

impl AceElement {
    /// TODO: add docs
    pub fn tag_name(&self) -> &str {
        &self.tag
    }
}
