use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


impl AceDOM {

    /// TODO: add docs
    pub fn get_node(&self, id: usize) -> Option<&AceNode> {
        self.nodes.get(id)
    }

    /// TODO: add docs
    pub fn get_node_mut(&mut self, id: usize) -> Option<&mut AceNode> {
        self.nodes.get_mut(id)
    }

    /// TODO: add docs
    pub fn get_element(&self, idx: usize) -> Option<&AceElement> {
        let node = self.get_node(idx)?;
        match &node.node_type {
            AceNodeType::Element(el) => Some(el),
            _ => None,
        }
    }

    /// TODO: add docs
    pub fn get_element_mut(&mut self, idx: usize) -> Option<&mut AceElement> {
        let node = self.get_node_mut(idx)?;
        match &mut node.node_type {
            AceNodeType::Element(el) => Some(el),
            _ => None,
        }
    }

    /// TODO: add docs
    pub fn create_text_node(&mut self, text: impl AsRef<str>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(AceNode {
            node_type: AceNodeType::Text(std::sync::Arc::from(text.as_ref())),
            parent: None,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE,
        });
        id
    }

    /// TODO: add docs
    pub fn create_comment_node(&mut self, text: impl AsRef<str>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(AceNode {
            node_type: AceNodeType::Comment(std::sync::Arc::from(text.as_ref())),
            parent: None,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE,
        });
        id
    }

    /// TODO: add docs
    pub fn create_element_node(&mut self, tag: impl AsRef<str>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(AceNode {
            node_type: AceNodeType::Element(AceElement {
                tag: tag.as_ref().to_string(),
                namespace: crate::ace::html::Namespace::Html,
                attributes: HashMap::new(),
            }),
            parent: None,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE,
        });
        id
    }

    /// DEPRECATED: Função removida junto com kuchiki
    #[deprecated(since = "1.1.0", note = "Use convert_html_node_recursive()")]
    pub fn convert_recursive(
        _kuchiki_node: &(),
        _nodes: &mut Vec<AceNode>,
        _parent_idx: Option<usize>,
    ) -> usize {
        panic!("convert_recursive() foi removido. Use convert_html_node_recursive() com HtmlNode do ACE-HTML parser.");
    }

pub(crate) fn convert_html_node_recursive(
        html_node: &HtmlNode,
        nodes: &mut Vec<AceNode>,
        parent_idx: Option<usize>,
    ) -> Option<usize> {
        let node_type = match html_node {
            HtmlNode::Element(element) => AceNodeType::Element(AceElement {
                tag: element.tag.clone(),
                namespace: element.namespace,
                attributes: element.attributes.clone(),
            }),
            HtmlNode::Text(text) => AceNodeType::Text(std::sync::Arc::from(text.as_str())),
            HtmlNode::Comment(text) => AceNodeType::Comment(std::sync::Arc::from(text.as_str())),
        };

        let current_idx = nodes.len();
        nodes.push(AceNode {
            node_type,
            parent: parent_idx,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE,
        });

        if let HtmlNode::Element(element) = html_node {
            let mut children = Vec::new();
            for child in &element.children {
                if let Some(child_idx) =
                    Self::convert_html_node_recursive(child, nodes, Some(current_idx))
                {
                    children.push(child_idx);
                }
            }
            Self::link_children(nodes, &children);
            if let Some(node) = nodes.get_mut(current_idx) {
                node.children = children;
            }
        }

        Some(current_idx)
    }
}
