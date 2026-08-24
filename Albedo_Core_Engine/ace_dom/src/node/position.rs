//! # Comparação de Posição Documental (WHATWG DOM §4.2)
//!
//! Implementação das bitflags e algoritmo normativo de `compareDocumentPosition`.

use crate::tree::Document;
use ace_core::id::NodeId;

bitflags::bitflags! {
    /// Máscara de bits representando a posição relativa de um nó em relação a outro.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct DocumentPosition: u16 {
        /// Os nós estão em documentos ou árvores desconectadas diferentes.
        const DISCONNECTED = 0x01;
        /// O outro nó precede este nó na ordem de documento.
        const PRECEDING = 0x02;
        /// O outro nó segue este nó na ordem de documento.
        const FOLLOWING = 0x04;
        /// O outro nó é um ancestral deste nó (contém este nó).
        const CONTAINS = 0x08;
        /// O outro nó é um descendente deste nó (está contido por este nó).
        const CONTAINED_BY = 0x10;
        /// A ordem relativa é específica da implementação (para nós desconectados).
        const IMPLEMENTATION_SPECIFIC = 0x20;
    }
}

/// Executa a comparação de posição documental normativa entre dois nós.
pub fn compare_document_position(doc: &Document, node_a: NodeId, node_b: NodeId) -> DocumentPosition {
    if node_a == node_b {
        return DocumentPosition::empty();
    }

    let node_a_data = match doc.get_node(node_a) {
        Some(n) => n,
        None => return DocumentPosition::DISCONNECTED | DocumentPosition::IMPLEMENTATION_SPECIFIC,
    };

    let node_b_data = match doc.get_node(node_b) {
        Some(n) => n,
        None => return DocumentPosition::DISCONNECTED | DocumentPosition::IMPLEMENTATION_SPECIFIC,
    };

    let _ = (node_a_data, node_b_data);

    // 1. Verifica se node_b é ancestral de node_a
    for (ancestor_id, _) in doc.ancestors(node_a) {
        if ancestor_id == node_b {
            return DocumentPosition::CONTAINS | DocumentPosition::PRECEDING;
        }
    }

    // 2. Verifica se node_a é ancestral de node_b
    for (ancestor_id, _) in doc.ancestors(node_b) {
        if ancestor_id == node_a {
            return DocumentPosition::CONTAINED_BY | DocumentPosition::FOLLOWING;
        }
    }

    // 3. Compara ordem em travessia pré-ordem na mesma árvore
    let mut pos_a = None;
    let mut pos_b = None;

    for (idx, (curr_id, _)) in doc.descendants(doc.root()).enumerate() {
        if curr_id == node_a {
            pos_a = Some(idx);
        }
        if curr_id == node_b {
            pos_b = Some(idx);
        }
        if pos_a.is_some() && pos_b.is_some() {
            break;
        }
    }

    match (pos_a, pos_b) {
        (Some(a), Some(b)) => {
            if b < a {
                DocumentPosition::PRECEDING
            } else {
                DocumentPosition::FOLLOWING
            }
        }
        _ => {
            // Nós estão desconectados da raiz do documento
            let fallback_order = if node_b.index() < node_a.index() {
                DocumentPosition::PRECEDING
            } else {
                DocumentPosition::FOLLOWING
            };
            DocumentPosition::DISCONNECTED | DocumentPosition::IMPLEMENTATION_SPECIFIC | fallback_order
        }
    }
}
