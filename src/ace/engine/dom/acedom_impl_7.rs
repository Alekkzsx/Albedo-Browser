use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


impl AceDOM {



pub(crate) fn escape_html(&self, s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str("&#39;"),
                _ => out.push(c),
            }
        }
        out
    }

pub(crate) fn escape_attr(&self, s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '&' => out.push_str("&amp;"),
                '"' => out.push_str("&quot;"),
                _ => out.push(c),
            }
        }
        out
    }

    /// TODO: add docs
    pub fn attach_shadow(&mut self, element_idx: usize) -> usize {
        let shadow_idx = self.nodes.len();
        self.nodes.push(AceNode {
            node_type: AceNodeType::ShadowRoot,
            parent: Some(element_idx),
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE,
        });

        if let Some(node) = self.nodes.get_mut(element_idx) {
            node.shadow_root = Some(shadow_idx);
        }

        self.mark_dirty(element_idx, NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE);

        shadow_idx
    }

    /// TODO: add docs
    pub fn observe(&mut self, target: usize, options: MutationObserverInit, callback_id: usize) {
        let entry = self.observers.entry(target).or_insert(Vec::new());
        entry.push(DomObserver {
            callback_id,
            options,
        });
    }

    /// TODO: add docs
    pub fn remove_attribute_notify(&mut self, node_idx: usize, name: String) {
        let mut old_value = None;
        if let Some(node) = self.nodes.get_mut(node_idx) {
            if let AceNodeType::Element(element) = &mut node.node_type {
                old_value = element.attributes.remove(&name);
            }
        }

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
    pub fn notify_mutation(&self, target: usize, record: MutationRecord) {
        // Collect observers that need to be notified
        // Logic:
        // 1. Check observers on target
        // 2. If bubbling (subtree: true), check ancestors

        // This is a simplified notification system that just prints or could invoke a callback mechanism
        // In a real implementation, this would interact with the JS runtime to queue a microtask

        let mut curr = Some(target);
        while let Some(node_idx) = curr {
            if let Some(observers) = self.observers.get(&node_idx) {
                for obs in observers {
                    let match_target = node_idx == target;
                    let match_subtree = obs.options.subtree;

                    if match_target || match_subtree {
                        match record.type_ {
                            MutationType::ChildList => {
                                if !obs.options.child_list {
                                    continue;
                                }
                            }
                            MutationType::Attributes => {
                                if !obs.options.attributes {
                                    continue;
                                }
                            }
                            MutationType::CharacterData => {
                                if !obs.options.character_data {
                                    continue;
                                }
                            }
                        }

                        let mut pending = self.pending_mutations_mut();
                        let entry = pending.entry(obs.callback_id).or_insert(Vec::new());
                        entry.push(record.clone());
                    }
                }
            }

            if let Some(node) = self.get_node(node_idx) {
                curr = node.parent;
            } else {
                curr = None;
            }
        }
    }
}
