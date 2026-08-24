//! # Índices Rápidos de Busca DOM (ElementIndex)
//!
//! Tabelas de dispersão de altíssimo throughput para resolução $O(1)$ de `getElementById`
//! e agrupamento instantâneo de classes.

use crate::node::NodeData;
use crate::tree::Document;
use ace_core::id::NodeId;
use ace_core::intern::Atom;
use rustc_hash::FxHashMap;

/// Índice de busca acelerada sobre os elementos de um `Document`.
#[derive(Debug, Clone, Default)]
pub struct ElementIndex {
    id_map: FxHashMap<Atom, NodeId>,
    class_map: FxHashMap<Atom, Vec<NodeId>>,
}

impl ElementIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reconstrói o índice completo a partir de um documento.
    pub fn rebuild(&mut self, doc: &Document) {
        self.id_map.clear();
        self.class_map.clear();

        for (node_id, node) in doc.descendants(doc.root()) {
            if let Some(el) = node.as_element() {
                // Mapeia ID
                if let Some(ref id_atom) = el.id_attr {
                    self.id_map.insert(id_atom.clone(), node_id);
                }

                // Mapeia Classes
                for class_atom in el.classes.as_slice() {
                    self.class_map
                        .entry(class_atom.clone())
                        .or_default()
                        .push(node_id);
                }
            }
        }
    }

    /// Indexa um nó adicionado.
    pub fn index_node(&mut self, node_id: NodeId, node: &NodeData) {
        if let Some(el) = node.as_element() {
            if let Some(ref id_atom) = el.id_attr {
                self.id_map.insert(id_atom.clone(), node_id);
            }
            for class_atom in el.classes.as_slice() {
                self.class_map
                    .entry(class_atom.clone())
                    .or_default()
                    .push(node_id);
            }
        }
    }

    /// Obtém o `NodeId` correspondente a um ID HTML em $O(1)$.
    #[inline]
    pub fn get_by_id(&self, id: &str) -> Option<NodeId> {
        let atom = Atom::new(id);
        self.id_map.get(&atom).copied()
    }

    /// Obtém todos os `NodeId`s que possuem a classe especificada.
    #[inline]
    pub fn get_by_class(&self, class_name: &str) -> &[NodeId] {
        let atom = Atom::new(class_name);
        self.class_map.get(&atom).map(|v| v.as_slice()).unwrap_or(&[])
    }
}
