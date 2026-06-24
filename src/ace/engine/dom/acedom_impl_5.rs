use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


impl AceDOM {

    /// TODO: add docs
    pub fn insert_before(&mut self, parent_idx: usize, child_idx: usize, ref_idx: Option<usize>) {
        self.remove_node_from_parent(child_idx);

        if let Some(parent) = self.nodes.get_mut(parent_idx) {
            let position = if let Some(r_idx) = ref_idx {
                parent
                    .children
                    .iter()
                    .position(|&idx| idx == r_idx)
                    .unwrap_or(parent.children.len())
            } else {
                parent.children.len()
            };

            parent.children.insert(position, child_idx);

            let prev = if position > 0 {
                Some(parent.children[position - 1])
            } else {
                None
            };
            let next = if position + 1 < parent.children.len() {
                Some(parent.children[position + 1])
            } else {
                None
            };

            if let Some(prev_idx) = prev {
                if let Some(prev_node) = self.nodes.get_mut(prev_idx) {
                    prev_node.next_sibling = Some(child_idx);
                }
            }

            if let Some(next_idx) = next {
                if let Some(next_node) = self.nodes.get_mut(next_idx) {
                    next_node.prev_sibling = Some(child_idx);
                }
            }

            if let Some(child_node) = self.nodes.get_mut(child_idx) {
                child_node.parent = Some(parent_idx);
                child_node.prev_sibling = prev;
                child_node.next_sibling = next;
            }
        }

        self.mark_dirty(
            parent_idx,
            NodeDirtyFlags::LAYOUT | NodeDirtyFlags::CHILDREN,
        );
        self.mark_dirty(child_idx, NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE);

        self.notify_mutation(
            parent_idx,
            MutationRecord {
                type_: MutationType::ChildList,
                target: parent_idx,
                added_nodes: vec![child_idx],
                removed_nodes: vec![],
                previous_sibling: if let Some(p) = self.get_node(child_idx) {
                    p.prev_sibling
                } else {
                    None
                },
                next_sibling: ref_idx,
                attribute_name: None,
                old_value: None,
            },
        );
    }

    /// DEPRECATED: Removido junto com kuchiki - use set_inner_html_from_nodes()
    #[deprecated(since = "1.1.0", note = "Use set_inner_html_from_nodes()")]
    pub fn set_inner_html_from_kuchiki(
        &mut self,
        _parent_idx: usize,
        _kuchiki_nodes: (),
    ) {
        panic!("set_inner_html_from_kuchiki() foi removido. Use set_inner_html_from_nodes() com HtmlNode do ACE-HTML parser.");
    }

    /// TODO: add docs
    pub fn set_inner_html_from_nodes(&mut self, parent_idx: usize, html_nodes: &[HtmlNode]) {
        let old_children = self
            .get_node(parent_idx)
            .map(|n| n.children.clone())
            .unwrap_or_default();
        if let Some(node) = self.nodes.get_mut(parent_idx) {
            node.children.clear();
        }

        let mut new_children = Vec::new();
        for child in html_nodes {
            if let Some(child_idx) =
                Self::convert_html_node_recursive(child, &mut self.nodes, Some(parent_idx))
            {
                new_children.push(child_idx);
            }
        }

        Self::link_children(&mut self.nodes, &new_children);

        if let Some(node) = self.nodes.get_mut(parent_idx) {
            node.children = new_children;
        }

        self.notify_mutation(
            parent_idx,
            MutationRecord {
                type_: MutationType::ChildList,
                target: parent_idx,
                added_nodes: self
                    .get_node(parent_idx)
                    .map(|n| n.children.clone())
                    .unwrap_or_default(),
                removed_nodes: old_children,
                previous_sibling: None,
                next_sibling: None,
                attribute_name: None,
                old_value: None,
            },
        );
    }
}
