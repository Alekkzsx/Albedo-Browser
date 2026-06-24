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



impl TreeSink for AceTreeSink {

pub(crate) fn is_mathml_annotation_xml_integration_point(&self, handle: &Self::Handle) -> bool {
        match &handle.data {
            AceSinkNodeData::Element {
                mathml_annotation_xml_integration_point,
                ..
            } => *mathml_annotation_xml_integration_point,
            _ => panic!("integration-point query on non-element"),
        }
    }

pub(crate) fn set_current_line(&self, line_number: u64) {
        self.current_line.set(line_number.max(1));
    }
}
