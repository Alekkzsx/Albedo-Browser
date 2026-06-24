use super::*;
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::mem;
use std::rc::{Rc, Weak};

use fxhash::FxHashSet;
use html5ever::tendril::StrTendril;
use html5ever::{Attribute, QualName};
use markup5ever::interface::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use smallvec::SmallVec;

use super::types::{
    DoctypeToken, HtmlDocument, HtmlElement, HtmlNode, Namespace, ParseError,
    ParseErrorKind, ParseErrorSource, ParseResult, ParseStats, ParserOptions,
};



/// TODO: add docs
pub fn serialize_children_as_text(children: &[HtmlNode]) -> String {
    let mut out = String::new();
    for child in children {
        match child {
            HtmlNode::Element(element) => {
                out.push('<');
                out.push_str(&element.tag);
                out.push('>');
                out.push_str(&serialize_children_as_text(&element.children));
                out.push_str("</");
                out.push_str(&element.tag);
                out.push('>');
            }
            HtmlNode::Text(text) | HtmlNode::Comment(text) => out.push_str(text),
        }
    }
    out
}
