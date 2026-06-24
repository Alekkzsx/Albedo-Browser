use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


impl AceDOM {

    /// TODO: add docs
    pub fn set_inner_html_from_html(&mut self, parent_idx: usize, html: &str) {
        let context = if let Some(node) = self.get_node(parent_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                Some(el.tag.as_str())
            } else {
                None
            }
        } else {
            None
        };
        let fragment = parse_fragment(html, context);
        self.set_inner_html_from_nodes(parent_idx, &fragment);
    }

    /// TODO: add docs
    pub fn import_html_fragment(&mut self, html: &str, parent_idx: Option<usize>) -> Vec<usize> {
        let context = parent_idx
            .and_then(|idx| self.get_node(idx))
            .and_then(|node| match &node.node_type {
                AceNodeType::Element(el) => Some(el.tag.clone()),
                _ => None,
            });
        let fragment = parse_fragment(html, context.as_deref());
        let mut imported = Vec::new();
        for node in &fragment {
            if let Some(idx) = Self::convert_html_node_recursive(node, &mut self.nodes, parent_idx)
            {
                imported.push(idx);
            }
        }
        Self::link_children(&mut self.nodes, &imported);
        imported
    }

    /// TODO: add docs
    pub fn set_text_content_notify(&mut self, node_idx: usize, text: String) {
        let old_children = self
            .get_node(node_idx)
            .map(|n| n.children.clone())
            .unwrap_or_default();

        if let Some(node) = self.nodes.get_mut(node_idx) {
            node.children.clear();
        }

        let text_idx = self.nodes.len();
        self.nodes.push(AceNode {
            node_type: AceNodeType::Text(std::sync::Arc::from(text)),
            parent: Some(node_idx),
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE,
        });

        if let Some(node) = self.nodes.get_mut(node_idx) {
            node.children.push(text_idx);
        }

        self.mark_dirty(node_idx, NodeDirtyFlags::LAYOUT | NodeDirtyFlags::CHILDREN);

        self.notify_mutation(
            node_idx,
            MutationRecord {
                type_: MutationType::ChildList,
                target: node_idx,
                added_nodes: vec![text_idx],
                removed_nodes: old_children,
                previous_sibling: None,
                next_sibling: None,
                attribute_name: None,
                old_value: None,
            },
        );
    }

    /// TODO: add docs
    pub fn serialize_subtree_text(&self, node_idx: usize) -> String {
        let mut s = String::new();
        if let Some(node) = self.get_node(node_idx) {
            match &node.node_type {
                AceNodeType::Text(t) => s.push_str(t),
                _ => {
                    for &child_idx in &node.children {
                        s.push_str(&self.serialize_subtree_text(child_idx));
                    }
                }
            }
        }
        s
    }

    /// TODO: add docs
    pub fn serialize_subtree_html(&self, node_idx: usize) -> String {
        let mut s = String::new();
        if let Some(node) = self.get_node(node_idx) {
            match &node.node_type {
                AceNodeType::Element(el) => {
                    s.push('<');
                    s.push_str(&el.tag);

                    let mut attrs: Vec<_> = el.attributes.iter().collect();
                    attrs.sort_by_key(|(k, _)| *k);

                    for (name, value) in attrs {
                        write!(s, " {}=\"{}\"", name, self.escape_attr(value)).expect("Albedo Engine: internal invariant violated");
                    }

                    if is_void_element(&el.tag) {
                        s.push('>');
                        return s;
                    }

                    s.push('>');
                    for &child_idx in &node.children {
                        s.push_str(&self.serialize_subtree_html(child_idx));
                    }
                    write!(s, "</{}>", el.tag).expect("Albedo Engine: internal invariant violated");
                }
                AceNodeType::Text(t) => {
                    s.push_str(&self.escape_html(t));
                }
                AceNodeType::Comment(c) => {
                    write!(s, "<!--{}-->", c).expect("Albedo Engine: internal invariant violated");
                }
                AceNodeType::Document | AceNodeType::DocumentFragment | AceNodeType::ShadowRoot => {
                    for &child_idx in &node.children {
                        s.push_str(&self.serialize_subtree_html(child_idx));
                    }
                }
            }
        }
        s
    }
}
