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

pub(crate) fn append_doctype_to_document(
        &self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        append_node(
            &self.document,
            AceSinkNode::new(AceSinkNodeData::Doctype {
                name,
                public_id,
                system_id,
            }),
        );
    }

pub(crate) fn get_template_contents(&self, target: &Self::Handle) -> Self::Handle {
        match &target.data {
            AceSinkNodeData::Element {
                template_contents, ..
            } => template_contents
                .borrow()
                .as_ref()
                .expect("template contents missing")
                .clone(),
            _ => panic!("get_template_contents called on non-template"),
        }
    }

pub(crate) fn same_node(&self, x: &Self::Handle, y: &Self::Handle) -> bool {
        Rc::ptr_eq(x, y)
    }

pub(crate) fn set_quirks_mode(&self, mode: QuirksMode) {
        self.quirks_mode.set(mode);
    }

pub(crate) fn append_before_sibling(&self, sibling: &Self::Handle, new_node: NodeOrText<Self::Handle>) {
        let (parent, index) =
            get_parent_and_index(sibling).expect("append_before_sibling called without parent");

        let child = match (new_node, index) {
            (NodeOrText::AppendText(text), 0) => AceSinkNode::new(AceSinkNodeData::Text {
                contents: RefCell::new(text),
            }),
            (NodeOrText::AppendText(text), idx) => {
                let children = parent.children.borrow();
                if append_to_existing_text(&children[idx - 1], &text) {
                    return;
                }
                drop(children);
                AceSinkNode::new(AceSinkNodeData::Text {
                    contents: RefCell::new(text),
                })
            }
            (NodeOrText::AppendNode(node), _) => node,
        };

        detach_from_parent(&child);
        *child.parent.borrow_mut() = Some(Rc::downgrade(&parent));
        parent.children.borrow_mut().insert(index, child);
    }

pub(crate) fn add_attrs_if_missing(&self, target: &Self::Handle, attrs: Vec<Attribute>) {
        let AceSinkNodeData::Element {
            attrs: existing, ..
        } = &target.data
        else {
            panic!("add_attrs_if_missing called on non-element");
        };

        let mut existing_attrs = existing.borrow_mut();
        let names = existing_attrs
            .iter()
            .map(|attr| attr.name.clone())
            .collect::<FxHashSet<_>>();

        existing_attrs.extend(attrs.into_iter().filter(|attr| !names.contains(&attr.name)));
    }

pub(crate) fn remove_from_parent(&self, target: &Self::Handle) {
        detach_from_parent(target);
    }

pub(crate) fn reparent_children(&self, node: &Self::Handle, new_parent: &Self::Handle) {
        let mut children = node.children.borrow_mut();
        let mut new_children = new_parent.children.borrow_mut();

        for child in children.iter() {
            let previous_parent = child.parent.replace(Some(Rc::downgrade(new_parent)));
            let previous_parent = previous_parent
                .expect("reparented child without parent")
                .upgrade()
                .expect("dangling parent pointer");
            assert!(Rc::ptr_eq(&previous_parent, node));
        }

        new_children.extend(mem::take(&mut *children));
    }
}
