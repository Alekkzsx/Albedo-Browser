//! # Estruturas de Registro de Mutação (WHATWG DOM §4.3)

use ace_core::id::NodeId;
use ace_core::intern::Atom;
use smol_str::SmolStr;

/// Tipo de mutação detectada na árvore DOM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationType {
    /// Mutação na lista de filhos (nós inseridos ou removidos)
    ChildList,
    /// Mutação em um atributo do elemento
    Attributes,
    /// Mutação no conteúdo textual de nós de texto/comentário
    CharacterData,
}

/// Registro imutável de uma alteração ocorrida na árvore DOM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationRecord {
    /// O tipo de mutação representada por este registro.
    pub record_type: MutationType,
    /// O nó afetado pela mutação.
    pub target: NodeId,
    /// Lista de nós adicionados durante a mutação.
    pub added_nodes: Vec<NodeId>,
    /// Lista de nós removidos durante a mutação.
    pub removed_nodes: Vec<NodeId>,
    /// Irmão imediatamente anterior ao nó modificado, se houver.
    pub previous_sibling: Option<NodeId>,
    /// Irmão imediatamente posterior ao nó modificado, se houver.
    pub next_sibling: Option<NodeId>,
    /// Nome do atributo alterado (se aplicável).
    pub attribute_name: Option<Atom>,
    /// Valor anterior do atributo ou dado textual antes da alteração (se solicitado).
    pub old_value: Option<SmolStr>,
}

impl MutationRecord {
    /// Cria um novo registro de mutação de filhos (`childList`).
    pub fn child_list(
        target: NodeId,
        added: Vec<NodeId>,
        removed: Vec<NodeId>,
        prev: Option<NodeId>,
        next: Option<NodeId>,
    ) -> Self {
        Self {
            record_type: MutationType::ChildList,
            target,
            added_nodes: added,
            removed_nodes: removed,
            previous_sibling: prev,
            next_sibling: next,
            attribute_name: None,
            old_value: None,
        }
    }

    /// Cria um novo registro de mutação de atributos.
    pub fn attribute(
        target: NodeId,
        name: Atom,
        old_value: Option<SmolStr>,
    ) -> Self {
        Self {
            record_type: MutationType::Attributes,
            target,
            added_nodes: Vec::new(),
            removed_nodes: Vec::new(),
            previous_sibling: None,
            next_sibling: None,
            attribute_name: Some(name),
            old_value,
        }
    }

    /// Cria um novo registro de mutação de texto (`characterData`).
    pub fn character_data(
        target: NodeId,
        old_value: Option<SmolStr>,
    ) -> Self {
        Self {
            record_type: MutationType::CharacterData,
            target,
            added_nodes: Vec::new(),
            removed_nodes: Vec::new(),
            previous_sibling: None,
            next_sibling: None,
            attribute_name: None,
            old_value,
        }
    }
}
