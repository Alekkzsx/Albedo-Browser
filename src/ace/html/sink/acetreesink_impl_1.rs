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
pub(crate) type Handle = Handle;
pub(crate) type Output = Self;
pub(crate) type ElemName<'a> = html5ever::ExpandedName<'a> where Self: 'a;

pub(crate) fn finish(self) -> Self::Output {
        self
    }

pub(crate) fn parse_error(&self, msg: Cow<'static, str>) {
        let line = self.current_line.get() as usize;
        self.parse_errors.borrow_mut().push(ParseError {
            code: "html5ever-parse-error".to_string(),
            source: ParseErrorSource::TreeBuilder,
            kind: ParseErrorKind::HtmlSyntax,
            line,
            column: 1,
            message: msg.into_owned(),
        });
    }

pub(crate) fn get_document(&self) -> Self::Handle {
        self.document.clone()
    }

pub(crate) fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> Self::ElemName<'a> {
        match &target.data {
            AceSinkNodeData::Element { name, .. } => name.expanded(),
            _ => panic!("elem_name called on non-element"),
        }
    }

pub(crate) fn create_element(
        &self,
        name: QualName,
        attrs: Vec<Attribute>,
        flags: ElementFlags,
    ) -> Self::Handle {
        AceSinkNode::new(AceSinkNodeData::Element {
            name,
            attrs: RefCell::new(SmallVec::from_vec(attrs)),
            template_contents: RefCell::new(if flags.template {
                Some(AceSinkNode::new(AceSinkNodeData::Document))
            } else {
                None
            }),
            mathml_annotation_xml_integration_point: flags.mathml_annotation_xml_integration_point,
        })
    }

pub(crate) fn create_comment(&self, text: StrTendril) -> Self::Handle {
        AceSinkNode::new(AceSinkNodeData::Comment { contents: text })
    }

pub(crate) fn create_pi(&self, target: StrTendril, data: StrTendril) -> Self::Handle {
        AceSinkNode::new(AceSinkNodeData::ProcessingInstruction {
            target,
            contents: data,
        })
    }

pub(crate) fn append(&self, parent: &Self::Handle, child: NodeOrText<Self::Handle>) {
        if let NodeOrText::AppendText(text) = &child {
            if let Some(last) = parent.children.borrow().last() {
                if append_to_existing_text(last, text) {
                    return;
                }
            }
        }

        append_node(
            parent,
            match child {
                NodeOrText::AppendNode(node) => node,
                NodeOrText::AppendText(text) => AceSinkNode::new(AceSinkNodeData::Text {
                    contents: RefCell::new(text),
                }),
            },
        );
    }

pub(crate) fn append_based_on_parent_node(
        &self,
        element: &Self::Handle,
        prev_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        if element.parent.borrow().is_some() {
            // Foster parenting error detection
            let mut errs = self.parse_errors.borrow_mut();
            if !errs.iter().any(|e| matches!(e.kind, super::types::ParseErrorKind::FosterParenting)) {
                errs.push(super::types::ParseError {
                    code: "foster-parenting".to_string(),
                    source: super::types::ParseErrorSource::TreeBuilder,
                    kind: super::types::ParseErrorKind::FosterParenting,
                    line: 1,
                    column: 1,
                    message: "Foster parenting occurred".to_string(),
                });
            }
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }
}
