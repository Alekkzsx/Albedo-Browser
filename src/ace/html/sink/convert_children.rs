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
pub fn convert_children(parent: &Handle, scripting_enabled: bool) -> Vec<HtmlNode> {
    parent
        .children
        .borrow()
        .iter()
        .filter_map(|child| convert_node(child, scripting_enabled))
        .collect()
}
