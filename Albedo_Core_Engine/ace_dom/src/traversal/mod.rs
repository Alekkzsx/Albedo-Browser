//! # Travessia e Iteração de Nós DOM (TreeWalker & NodeIterator — WHATWG DOM §6)
//!
//! Fornece navegadores estruturados de alta performance sobre subárvores DOM com filtros de tipo.

pub mod filter;

pub use filter::{FilterResult, NodeFilter};

use crate::node::NodeKind;
use crate::tree::Document;
use ace_core::id::NodeId;

/// Testa se um nó satisfaz o filtro de tipos `NodeFilter`.
fn matches_filter(doc: &Document, node_id: NodeId, filter: NodeFilter) -> bool {
    let node = match doc.get_node(node_id) {
        Some(n) => n,
        None => return false,
    };

    match &node.kind {
        NodeKind::Element(_) => filter.contains(NodeFilter::SHOW_ELEMENT),
        NodeKind::Text(_) => filter.contains(NodeFilter::SHOW_TEXT),
        NodeKind::Comment(_) => filter.contains(NodeFilter::SHOW_COMMENT),
        NodeKind::Document(_) => filter.contains(NodeFilter::SHOW_DOCUMENT),
        NodeKind::DocumentType(_) => filter.contains(NodeFilter::SHOW_DOCUMENT_TYPE),
        NodeKind::DocumentFragment => filter.contains(NodeFilter::SHOW_DOCUMENT_FRAGMENT),
        NodeKind::ShadowRoot(_) => filter.contains(NodeFilter::SHOW_SHADOW_ROOT),
    }
}

/// Helper para obter a lista de nós na ordem canônica do documento (raiz + descendentes).
fn document_order_nodes(doc: &Document, root: NodeId) -> Vec<NodeId> {
    let mut nodes = Vec::with_capacity(16);
    nodes.push(root);
    for (child_id, _) in doc.descendants(root) {
        nodes.push(child_id);
    }
    nodes
}

/// O `TreeWalker` oficial para navegação bidirecional na árvore DOM.
#[derive(Debug, Clone)]
pub struct TreeWalker {
    pub root: NodeId,
    pub what_to_show: NodeFilter,
    pub current_node: NodeId,
}

impl TreeWalker {
    /// Cria um novo `TreeWalker` a partir de uma raiz e um filtro de tipos.
    pub fn new(root: NodeId, what_to_show: NodeFilter) -> Self {
        Self {
            root,
            what_to_show,
            current_node: root,
        }
    }

    /// Move o cursor para o nó pai e o retorna.
    pub fn parent_node(&mut self, doc: &Document) -> Option<NodeId> {
        let mut node = self.current_node;
        while node != self.root {
            if let Some(parent) = doc.get_node(node).and_then(|n| n.parent) {
                node = parent;
                if matches_filter(doc, node, self.what_to_show) {
                    self.current_node = node;
                    return Some(node);
                }
            } else {
                break;
            }
        }
        None
    }

    /// Move o cursor para o primeiro filho válido e o retorna.
    pub fn first_child(&mut self, doc: &Document) -> Option<NodeId> {
        let node = doc.get_node(self.current_node)?;
        let mut child = node.first_child;

        while let Some(c_id) = child {
            if matches_filter(doc, c_id, self.what_to_show) {
                self.current_node = c_id;
                return Some(c_id);
            }
            child = doc.get_node(c_id).and_then(|n| n.next_sibling);
        }
        None
    }

    /// Move o cursor para o próximo nó em ordem DFS e o retorna.
    pub fn next_node(&mut self, doc: &Document) -> Option<NodeId> {
        let nodes = document_order_nodes(doc, self.root);
        let mut found = false;

        for node_id in nodes {
            if found {
                if matches_filter(doc, node_id, self.what_to_show) {
                    self.current_node = node_id;
                    return Some(node_id);
                }
            } else if node_id == self.current_node {
                found = true;
            }
        }
        None
    }
}

/// O `NodeIterator` oficial para iteração linear de nós DOM (WHATWG DOM §6).
#[derive(Debug, Clone)]
pub struct NodeIterator {
    pub root: NodeId,
    pub what_to_show: NodeFilter,
    pub reference_node: NodeId,
    pub pointer_before_reference_node: bool,
}

impl NodeIterator {
    /// Cria um novo `NodeIterator`.
    pub fn new(root: NodeId, what_to_show: NodeFilter) -> Self {
        Self {
            root,
            what_to_show,
            reference_node: root,
            pointer_before_reference_node: true,
        }
    }

    /// Avança e retorna o próximo nó que satisfaz o filtro.
    pub fn next_node(&mut self, doc: &Document) -> Option<NodeId> {
        let nodes = document_order_nodes(doc, self.root);
        let mut passed_ref = false;

        for node_id in nodes {
            if passed_ref {
                if matches_filter(doc, node_id, self.what_to_show) {
                    self.reference_node = node_id;
                    self.pointer_before_reference_node = false;
                    return Some(node_id);
                }
            } else if node_id == self.reference_node {
                if self.pointer_before_reference_node {
                    self.pointer_before_reference_node = false;
                    if matches_filter(doc, node_id, self.what_to_show) {
                        return Some(node_id);
                    }
                } else {
                    passed_ref = true;
                }
            }
        }
        None
    }

    /// Retrocede e retorna o nó anterior que satisfaz o filtro.
    pub fn previous_node(&mut self, doc: &Document) -> Option<NodeId> {
        let nodes = document_order_nodes(doc, self.root);
        let mut passed_ref = false;

        for node_id in nodes.into_iter().rev() {
            if passed_ref {
                if matches_filter(doc, node_id, self.what_to_show) {
                    self.reference_node = node_id;
                    self.pointer_before_reference_node = true;
                    return Some(node_id);
                }
            } else if node_id == self.reference_node {
                if !self.pointer_before_reference_node {
                    self.pointer_before_reference_node = true;
                    if matches_filter(doc, node_id, self.what_to_show) {
                        return Some(node_id);
                    }
                } else {
                    passed_ref = true;
                }
            }
        }
        None
    }
}
