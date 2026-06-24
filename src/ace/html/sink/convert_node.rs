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
pub fn convert_node(node: &Handle, scripting_enabled: bool) -> Option<HtmlNode> {
    match &node.data {
        AceSinkNodeData::Document | AceSinkNodeData::Doctype { .. } => None,
        AceSinkNodeData::Text { contents } => Some(HtmlNode::Text(contents.borrow().to_string())),
        AceSinkNodeData::Comment { contents } => Some(HtmlNode::Comment(contents.to_string())),
        AceSinkNodeData::ProcessingInstruction { target, contents } => {
            Some(HtmlNode::Comment(format!("?{} {}", target, contents)))
        }
        AceSinkNodeData::Element {
            name,
            attrs,
            template_contents,
            ..
        } => {
            let mut children = convert_children(node, scripting_enabled);

            if let Some(contents) = template_contents.borrow().as_ref() {
                children.insert(
                    0,
                    HtmlNode::Element(HtmlElement {
                        tag: "template-content".to_string(),
                        namespace: Namespace::Html,
                        attributes: std::collections::HashMap::new(),
                        children: convert_children(contents, scripting_enabled),
                    }),
                );
            }

            let tag = name.local.to_string();
            if tag == "noscript" && scripting_enabled {
                let raw = serialize_children_as_text(&children);
                children = vec![HtmlNode::Text(raw)];
            }

            let attributes = attrs
                .borrow()
                .iter()
                .map(|attr| (attribute_name_to_string(&attr.name), attr.value.to_string()))
                .collect();

            Some(HtmlNode::Element(HtmlElement {
                tag,
                namespace: map_namespace(name),
                attributes,
                children,
            }))
        }
    }
}
