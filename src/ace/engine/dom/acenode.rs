use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


#[derive(Clone, Debug)]
pub struct AceNode {
    pub node_type: AceNodeType,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub prev_sibling: Option<usize>,
    pub next_sibling: Option<usize>,
    pub shadow_root: Option<usize>, // FASE 5: Shadow DOM support
    pub dirty: NodeDirtyFlags,
}

impl AceNode {
    /// TODO: add docs
    pub fn get_text_content(&self) -> String {
        match &self.node_type {
            AceNodeType::Text(text) => text.to_string(),
            _ => String::new(),
        }
    }

    /// TODO: add docs
    pub fn node_type(&self) -> NodeType {
        match self.node_type {
            AceNodeType::Element(_) => NodeType::Element,
            AceNodeType::Text(_) => NodeType::Text,
            AceNodeType::Comment(_) => NodeType::Comment,
            AceNodeType::Document => NodeType::Document,
            AceNodeType::ShadowRoot => NodeType::ShadowRoot,
            AceNodeType::DocumentFragment => NodeType::DocumentFragment,
        }
    }

    /// TODO: add docs
    pub fn text_content(&self) -> Option<&str> {
        match &self.node_type {
            AceNodeType::Text(text) | AceNodeType::Comment(text) => Some(text),
            _ => None,
        }
    }

    /// TODO: add docs
    pub fn set_text_content(&mut self, text: &str) {
        match &mut self.node_type {
            AceNodeType::Text(current) | AceNodeType::Comment(current) => {
                *current = std::sync::Arc::from(text);
            }
            _ => {}
        }
    }

    /// TODO: add docs
    pub fn parent(&self) -> Option<NodeRef> {
        self.parent.map(|id| NodeRef { id })
    }

    /// TODO: add docs
    pub fn children(&self) -> &[usize] {
        &self.children
    }
}
