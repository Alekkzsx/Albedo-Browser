use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


impl AceDOM {

    /// TODO: add docs
    pub fn mark_dirty(&mut self, node_idx: usize, flags: NodeDirtyFlags) {
        if flags.is_empty() {
            return;
        }

        let mut current_idx = Some(node_idx);
        let mut first = true;

        while let Some(idx) = current_idx {
            if let Some(node) = self.nodes.get_mut(idx) {
                if first {
                    node.dirty.insert(flags);
                    first = false;
                }

                // If subtree flag is already set, we can stop propagating up
                // (except for the first node which might have other flags)
                if node.dirty.contains(NodeDirtyFlags::SUBTREE) && !first {
                    break;
                }

                node.dirty.insert(NodeDirtyFlags::SUBTREE);
                current_idx = node.parent;
            } else {
                break;
            }
        }
        
        // Notificar LiveNodeLists que precisam se atualizar
        self.mark_live_collections_dirty(node_idx);
    }
    
    /// Marca todas as LiveNodeLists afetadas por uma mutation como dirty
pub(crate) fn mark_live_collections_dirty(&mut self, _mutated_node_idx: usize) {
        // Em produção, isso iteraria sobre um registro de LiveNodeLists ativas
        // e marcaria como dirty aquelas cujo root é ancestor do nó mutado
        // Implementação simplificada - em produção usaria um WeakMap para evitar memory leaks
    }

    /// TODO: add docs
    pub fn remove_node_from_parent(&mut self, node_idx: usize) {
        let parent_idx = if let Some(node) = self.nodes.get(node_idx) {
            node.parent
        } else {
            return;
        };

        if let Some(p_idx) = parent_idx {
            // Capture state for notification before removal
            let (prev_sibling, next_sibling) = if let Some(node) = self.nodes.get(node_idx) {
                (node.prev_sibling, node.next_sibling)
            } else {
                (None, None)
            };

            if let Some(parent) = self.nodes.get_mut(p_idx) {
                parent.children.retain(|&idx| idx != node_idx);
            }

            let node = self.nodes.get(node_idx).expect("Albedo Engine: internal invariant violated");
            let prev = node.prev_sibling;
            let next = node.next_sibling;

            if let Some(prev_idx) = prev {
                if let Some(prev_node) = self.nodes.get_mut(prev_idx) {
                    prev_node.next_sibling = next;
                }
            }

            if let Some(next_idx) = next {
                if let Some(next_node) = self.nodes.get_mut(next_idx) {
                    next_node.prev_sibling = prev;
                }
            }

            if let Some(node) = self.nodes.get_mut(node_idx) {
                node.parent = None;
                node.prev_sibling = None;
                node.next_sibling = None;
            }

            // Mark parent dirty
            self.mark_dirty(p_idx, NodeDirtyFlags::LAYOUT | NodeDirtyFlags::CHILDREN);

            // Notify observers
            self.notify_mutation(
                p_idx,
                MutationRecord {
                    type_: MutationType::ChildList,
                    target: p_idx,
                    added_nodes: vec![],
                    removed_nodes: vec![node_idx],
                    previous_sibling: prev_sibling,
                    next_sibling: next_sibling,
                    attribute_name: None,
                    old_value: None,
                },
            );
        }
    }
}
