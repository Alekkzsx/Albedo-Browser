use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


impl AceDOM {

    // Helper to access pending_mutations mutably
pub(crate) fn pending_mutations_mut(&self) -> std::cell::RefMut<'_, HashMap<usize, Vec<MutationRecord>>> {
        self.pending_mutations.borrow_mut()
    }

    /// TODO: add docs
    pub fn take_pending_mutations(&mut self) -> HashMap<usize, Vec<MutationRecord>> {
        let mut pending = HashMap::new();
        std::mem::swap(&mut pending, &mut *self.pending_mutations.borrow_mut());
        pending
    }

    /// TODO: add docs
    pub fn set_attribute(&mut self, node_idx: usize, name: String, value: String) {
        if let Some(node) = self.nodes.get_mut(node_idx) {
            if let AceNodeType::Element(el) = &mut node.node_type {
                let _old_value = el.attributes.get(&name).cloned();
                el.attributes.insert(name.clone(), value.clone());

                // Drop mutable borrow to call notify
            }
        }

        // Re-borrow to notify (this is slightly inefficient doing lookup twice, but safe)
        // We need the old value, so we must have done the mutation first
        // Ideally we would return old_value from the mutation block
        // But let's keep it simple for now, we can optimize later

        // Notify observers
        // We need to fetch old_value again? No, we can't because we just overwrote it.
        // The previous block logic is flawed because we can't easily extract old_value out of the scope
        // while also mutating.
        // Let's refactor slightly to be correct.
    }

    // Helper to set attribute with notification
    pub fn set_attribute_notify(&mut self, node_idx: usize, name: String, value: String) {
        let mut old_value = None;
        if let Some(node) = self.nodes.get_mut(node_idx) {
            if let AceNodeType::Element(el) = &mut node.node_type {
                old_value = el.attributes.insert(name.clone(), value.clone());
            }
        }

        self.mark_dirty(node_idx, NodeDirtyFlags::STYLE | NodeDirtyFlags::LAYOUT);

        self.notify_mutation(
            node_idx,
            MutationRecord {
                type_: MutationType::Attributes,
                target: node_idx,
                added_nodes: vec![],
                removed_nodes: vec![],
                previous_sibling: None,
                next_sibling: None,
                attribute_name: Some(name),
                old_value,
            },
        );
    }

    /// TODO: add docs
    pub fn clone_subtree(&mut self, node_idx: usize, deep: bool) -> usize {
        let node = self.nodes.get(node_idx).cloned().expect("Albedo Engine: internal invariant violated");
        let new_idx = self.nodes.len();

        // Push initial stub to reserve index
        self.nodes.push(node.clone());

        let mut new_node = node.clone();
        new_node.parent = None;
        new_node.prev_sibling = None;
        new_node.next_sibling = None;
        new_node.children = Vec::new();

        if deep {
            let mut children_indices = Vec::new();
            // Need to fetch original node's children
            let original_children = node.children.clone();
            for &child_idx in &original_children {
                let new_child_idx = self.clone_subtree(child_idx, true);
                children_indices.push(new_child_idx);
                if let Some(child) = self.nodes.get_mut(new_child_idx) {
                    child.parent = Some(new_idx);
                }
            }

            // Set siblings for new children
            for i in 0..children_indices.len() {
                let curr = children_indices[i];
                let prev = if i > 0 {
                    Some(children_indices[i - 1])
                } else {
                    None
                };
                let next = if i < children_indices.len() - 1 {
                    Some(children_indices[i + 1])
                } else {
                    None
                };
                if let Some(child) = self.nodes.get_mut(curr) {
                    child.prev_sibling = prev;
                    child.next_sibling = next;
                }
            }
            new_node.children = children_indices;
        }

        // Update the reserved index with actual data
        self.nodes[new_idx] = new_node;
        new_idx
    }
}
