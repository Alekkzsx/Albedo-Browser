use super::*;
use std::collections::{BTreeMap, HashMap};



#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HtmlTokenKind {
    StartTag(StartTagToken),
    EndTag(EndTagToken),
    Character(CharacterToken),
    Comment(CommentToken),
    Doctype(DoctypeToken),
    Eof,
}
