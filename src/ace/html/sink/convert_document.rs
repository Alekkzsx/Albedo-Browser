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
pub fn convert_document(
    document: &Handle,
    scripting_enabled: bool,
    quirks_mode: QuirksMode,
) -> (Option<DoctypeToken>, Vec<HtmlNode>) {
    let mut doctype = None;
    let mut children = Vec::new();

    for child in document.children.borrow().iter() {
        match &child.data {
            AceSinkNodeData::Doctype {
                name,
                public_id,
                system_id,
            } if doctype.is_none() => {
                doctype = Some(DoctypeToken {
                    name: Some(name.to_string()),
                    public_id: if public_id.is_empty() {
                        None
                    } else {
                        Some(public_id.to_string())
                    },
                    system_id: if system_id.is_empty() {
                        None
                    } else {
                        Some(system_id.to_string())
                    },
                    force_quirks: matches!(quirks_mode, QuirksMode::Quirks),
                });
            }
            _ => {
                if let Some(node) = convert_node(child, scripting_enabled) {
                    children.push(node);
                }
            }
        }
    }

    (doctype, children)
}
