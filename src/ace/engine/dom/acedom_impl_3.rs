use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


impl AceDOM {

pub(crate) fn link_children(nodes: &mut [AceNode], children: &[usize]) {
        for (i, &curr) in children.iter().enumerate() {
            let prev = if i > 0 { Some(children[i - 1]) } else { None };
            let next = if i + 1 < children.len() {
                Some(children[i + 1])
            } else {
                None
            };

            if let Some(node) = nodes.get_mut(curr) {
                node.prev_sibling = prev;
                node.next_sibling = next;
            }
        }
    }

pub(crate) fn find_head_body(&mut self) {
        // Busca simples a partir da raiz
        if let Some(root_node) = self.nodes.get(self.root) {
            for &child_idx in &root_node.children {
                if let Some(html_node) = self.nodes.get(child_idx) {
                    if let AceNodeType::Element(el) = &html_node.node_type {
                        if el.tag == "html" {
                            for &grandchild_idx in &html_node.children {
                                if let Some(grandchild) = self.nodes.get(grandchild_idx) {
                                    if let AceNodeType::Element(gc_el) = &grandchild.node_type {
                                        if gc_el.tag == "head" {
                                            self.head = Some(grandchild_idx);
                                        } else if gc_el.tag == "body" {
                                            self.body = Some(grandchild_idx);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if self.head.is_none() || self.body.is_none() {
            for (i, node) in self.nodes.iter().enumerate() {
                if let AceNodeType::Element(el) = &node.node_type {
                    if self.head.is_none() && el.tag == "head" {
                        self.head = Some(i);
                    }
                    if self.body.is_none() && el.tag == "body" {
                        self.body = Some(i);
                    }
                }
            }
        }
    }

    /// TODO: add docs
    pub fn append_child(&mut self, parent_idx: usize, child_idx: usize) {
        self.remove_node_from_parent(child_idx);

        let mut prev_sibling = None;

        if let Some(parent) = self.nodes.get_mut(parent_idx) {
            let last_child = parent.children.last().cloned();
            prev_sibling = last_child;
            parent.children.push(child_idx);

            if let Some(last_idx) = last_child {
                if let Some(last_node) = self.nodes.get_mut(last_idx) {
                    last_node.next_sibling = Some(child_idx);
                }
            }

            if let Some(child_node) = self.nodes.get_mut(child_idx) {
                child_node.parent = Some(parent_idx);
                child_node.prev_sibling = last_child;
                child_node.next_sibling = None;
            }
        }

        // Propagate dirty flags
        self.mark_dirty(
            parent_idx,
            NodeDirtyFlags::LAYOUT | NodeDirtyFlags::CHILDREN,
        );
        self.mark_dirty(child_idx, NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE);

        // Notify observers
        self.notify_mutation(
            parent_idx,
            MutationRecord {
                type_: MutationType::ChildList,
                target: parent_idx,
                added_nodes: vec![child_idx],
                removed_nodes: vec![],
                previous_sibling: prev_sibling,
                next_sibling: None,
                attribute_name: None,
                old_value: None,
            },
        );
    }
}
