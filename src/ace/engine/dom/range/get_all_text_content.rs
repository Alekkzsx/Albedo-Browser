use super::*;
// AceDOM - Range API Implementation
// FASE 2: Range API completa (W3C DOM Range spec)
// Status: 100% implementado e documentado

use crate::ace::engine::dom::{AceDOM, NodeId, NodeType};

/// Ponto de limite (boundary point) no Range


pub(crate) fn get_all_text_content(node_idx: NodeId, dom: &AceDOM) -> String {
    let mut text = String::new();
    if let Some(node) = dom.get_node(node_idx) {
        match &node.node_type {
            crate::ace::engine::dom::AceNodeType::Text(t) => text.push_str(t),
            _ => {
                for &child in &node.children {
                    text.push_str(&get_all_text_content(child, dom));
                }
            }
        }
    }
    text
}
