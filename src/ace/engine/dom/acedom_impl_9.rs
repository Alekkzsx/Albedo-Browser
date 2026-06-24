use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


impl AceDOM {

    /// TODO: add docs
    pub fn insert_adjacent_html(&mut self, target_idx: usize, position: &str, html: &str) {
        let insertion_position = position.to_lowercase();
        let parse_context_parent = match insertion_position.as_str() {
            "beforebegin" | "afterend" => self.get_node(target_idx).and_then(|n| n.parent),
            "afterbegin" | "beforeend" => Some(target_idx),
            _ => None,
        };

        let imported_indices = self.import_html_fragment(html, parse_context_parent);

        if imported_indices.is_empty() {
            return;
        }

        // Determinar onde inserir baseado na posição
        match insertion_position.as_str() {
            "beforebegin" => {
                let parent = self.get_node(target_idx).and_then(|n| n.parent);
                if let Some(p_idx) = parent {
                    for idx in imported_indices {
                        self.insert_before(p_idx, idx, Some(target_idx));
                    }
                }
            }
            "afterbegin" => {
                let first_child = self
                    .get_node(target_idx)
                    .and_then(|n| n.children.first().cloned());
                for idx in imported_indices.into_iter().rev() {
                    self.insert_before(target_idx, idx, first_child);
                }
            }
            "beforeend" => {
                for idx in imported_indices {
                    self.append_child(target_idx, idx);
                }
            }
            "afterend" => {
                let parent = self.get_node(target_idx).and_then(|n| n.parent);
                if let Some(p_idx) = parent {
                    let next_sibling = self.get_node(target_idx).and_then(|n| n.next_sibling);
                    for idx in imported_indices.into_iter().rev() {
                        self.insert_before(p_idx, idx, next_sibling);
                    }
                }
            }
            _ => {}
        }
    }
}
