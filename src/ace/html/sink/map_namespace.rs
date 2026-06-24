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
pub fn map_namespace(name: &QualName) -> Namespace {
    use html5ever::ns;
    if name.ns == ns!(svg) {
        Namespace::Svg
    } else if name.ns == ns!(mathml) {
        Namespace::MathMl
    } else {
        Namespace::Html
    }
}
