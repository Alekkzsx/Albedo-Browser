use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


#[derive(Clone, Debug)]
pub struct DomObserver {
    pub callback_id: usize, // ID for JS callback
    pub options: MutationObserverInit,
}
