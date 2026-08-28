//! # Pipeline de Observação de Mutações (MutationObserver — WHATWG DOM §4.3)
//!
//! Permite a observação assíncrona e em lote de alterações na árvore DOM.

pub mod record;

pub use record::{MutationRecord, MutationType};

use ace_core::id::NodeId;
use ace_core::intern::Atom;
use rustc_hash::FxHashMap;

/// Opções de configuração para o `MutationObserver`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MutationObserverInit {
    /// Observar adição e remoção de nós filhos imediatos.
    pub child_list: bool,
    /// Observar alterações em atributos.
    pub attributes: bool,
    /// Observar alterações em dados de texto/comentário.
    pub character_data: bool,
    /// Estender a observação recursivamente para todos os descendentes.
    pub subtree: bool,
    /// Gravar o valor anterior do atributo antes da mutação.
    pub attribute_old_value: bool,
    /// Gravar o valor anterior do dado textual antes da mutação.
    pub character_data_old_value: bool,
    /// Lista de atributos específicos a observar (se vazio, observa todos).
    pub attribute_filter: Option<Vec<Atom>>,
}

/// Observador oficial de mutações da árvore DOM.
#[derive(Debug, Default)]
pub struct MutationObserver {
    /// Nós alvos observados mapeados para suas respectivas opções de observação.
    targets: FxHashMap<NodeId, MutationObserverInit>,
    /// Fila de registros de mutação pendentes de entrega.
    record_queue: Vec<MutationRecord>,
}

impl MutationObserver {
    /// Cria uma nova instância de `MutationObserver`.
    pub fn new() -> Self {
        Self {
            targets: FxHashMap::default(),
            record_queue: Vec::with_capacity(32),
        }
    }

    /// Inicia ou atualiza a observação sobre um determinado nó da árvore.
    pub fn observe(&mut self, target: NodeId, options: MutationObserverInit) {
        self.targets.insert(target, options);
    }

    /// Encerra todas as observações ativas deste observador.
    pub fn disconnect(&mut self) {
        self.targets.clear();
        self.record_queue.clear();
    }

    /// Retorna e esvazia a fila de registros de mutações pendentes.
    pub fn take_records(&mut self) -> Vec<MutationRecord> {
        std::mem::take(&mut self.record_queue)
    }

    fn matches_filter(options: &MutationObserverInit, record: &MutationRecord) -> bool {
        match record.record_type {
            MutationType::ChildList => options.child_list,
            MutationType::Attributes => {
                if !options.attributes {
                    return false;
                }
                if let Some(ref filter) = options.attribute_filter {
                    if let Some(ref attr_name) = record.attribute_name {
                        filter.contains(attr_name)
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
            MutationType::CharacterData => options.character_data,
        }
    }

    /// Notifica o observador sobre uma mutação ocorrida, enfileirando o registro se aplicável.
    pub fn notify_mutation(&mut self, record: MutationRecord) {
        if let Some(options) = self.targets.get(&record.target) {
            if Self::matches_filter(options, &record) {
                self.record_queue.push(record);
            }
        }
    }

    /// Notifica o observador sobre uma mutação ocorrida, checando também ancestrais com `subtree: true`.
    pub fn notify_mutation_tree(&mut self, doc: &crate::tree::Document, record: MutationRecord) {
        // 1. Checa o alvo direto
        if let Some(options) = self.targets.get(&record.target) {
            if Self::matches_filter(options, &record) {
                self.record_queue.push(record);
                return;
            }
        }

        // 2. Checa ancestrais se subtree: true
        for (ancestor_id, _) in doc.ancestors(record.target) {
            if let Some(options) = self.targets.get(&ancestor_id) {
                if options.subtree && Self::matches_filter(options, &record) {
                    self.record_queue.push(record);
                    return;
                }
            }
        }
    }
}
