//! # Resolução de Árvore Achatada e Distribuição de Slots (Shadow DOM Flat Tree)
//!
//! Implementação do algoritmo normativo de atribuição de slots (WHATWG DOM §4.2.2.3)
//! e navegação na árvore composta (*Composed/Flat Tree*) para renderização e cascata de estilo.

use crate::tree::Document;
use ace_core::id::NodeId;

/// Algoritmo de distribuição de slots e resolução da Flat Tree.
pub struct FlatTreeResolver;

impl FlatTreeResolver {
    /// Retorna os nós atribuídos a um elemento `<slot>` específico.
    pub fn get_assigned_nodes(doc: &Document, slot_id: NodeId) -> Vec<NodeId> {
        let slot_node = match doc.get_node(slot_id) {
            Some(n) => n,
            None => return Vec::new(),
        };

        let slot_el = match slot_node.as_element() {
            Some(e) if e.tag_name.eq_ignore_ascii_case("slot") => e,
            _ => return Vec::new(),
        };

        let target_slot_name = slot_el.get_attribute("name");

        // Encontra o hospedeiro da ShadowRoot pai
        let mut host_id = None;
        for (ancestor_id, ancestor_node) in doc.ancestors(slot_id) {
            if let crate::node::NodeKind::ShadowRoot(ref s_data) = ancestor_node.kind {
                host_id = Some(s_data.host);
                break;
            }
            let _ = ancestor_id;
        }

        let host_id = match host_id {
            Some(h) => h,
            None => return Vec::new(),
        };

        // Varre os filhos diretos do hospedeiro (os slotables)
        let mut assigned = Vec::new();
        for (child_id, child_node) in doc.children(host_id) {
            let child_slot_name = child_node
                .as_element()
                .and_then(|el| el.get_attribute("slot"));

            match (target_slot_name, child_slot_name) {
                // Named slot match
                (Some(expected), Some(actual)) if expected == actual => {
                    assigned.push(child_id);
                }
                // Default slot match (ambos sem nome)
                (None, None) => {
                    assigned.push(child_id);
                }
                _ => {}
            }
        }

        // Se não houver nós atribuídos, retorna o conteúdo de fallback do próprio slot
        if assigned.is_empty() {
            for (fallback_id, _) in doc.children(slot_id) {
                assigned.push(fallback_id);
            }
        }

        assigned
    }

    /// Retorna a lista de nós filhos imediatos na visão achatada (*Flat Tree*).
    pub fn flat_tree_children(doc: &Document, node_id: NodeId) -> Vec<NodeId> {
        let node = match doc.get_node(node_id) {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Se o nó for um hospedeiro com ShadowRoot anexada, desce na ShadowRoot
        if let Some(el) = node.as_element() {
            if let Some(shadow_root_id) = el.shadow_root {
                return Self::flat_tree_children(doc, shadow_root_id);
            }

            // Se for um slot, retorna os nós distribuídos (ou fallback)
            if el.tag_name.eq_ignore_ascii_case("slot") {
                return Self::get_assigned_nodes(doc, node_id);
            }
        }

        // Caso padrão: filhos normais da árvore
        let mut children = Vec::new();
        for (child_id, _) in doc.children(node_id) {
            children.push(child_id);
        }
        children
    }
}
