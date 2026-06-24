use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutationType {
    ChildList,
    Attributes,
    CharacterData,
}

impl MutationType {
    /// TODO: add docs
    pub fn as_str(&self) -> &'static str {
        match self {
            MutationType::ChildList => "childList",
            MutationType::Attributes => "attributes",
            MutationType::CharacterData => "characterData",
        }
    }
}
