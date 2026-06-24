use super::*;
use std::collections::{BTreeMap, HashMap};



#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HtmlNode {
    Element(HtmlElement),
    Text(String),
    Comment(String),
}
