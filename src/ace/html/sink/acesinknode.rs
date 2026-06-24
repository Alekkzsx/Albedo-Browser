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



#[derive(Debug)]
pub struct AceSinkNode {
    pub parent: RefCell<Option<WeakHandle>>,
    pub children: RefCell<Vec<Handle>>,
    pub data: AceSinkNodeData,
}

impl AceSinkNode {
    /// TODO: add docs
    pub fn new(data: AceSinkNodeData) -> Handle {
        Rc::new(Self {
            parent: RefCell::new(None),
            children: RefCell::new(Vec::new()),
            data,
        })
    }
}
