use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


#[derive(Clone, Debug, PartialEq)]
pub enum AceNodeType {
    Element(AceElement),
    Text(std::sync::Arc<str>),
    Comment(std::sync::Arc<str>),
    Document,
    ShadowRoot, // FASE 5: Shadow DOM root
    DocumentFragment,
}
