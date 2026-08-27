//! # Pilha de Reações de Elementos Customizados (WHATWG HTML §4.13.3)
//!
//! Gerenciamento da pilha de reações de ciclo de vida (`[CEReactions]`) e filas
//! de processamento para elementos customizados.

use ace_core::arena::{Arena, ArenaId};
use ace_core::id::NodeId;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::collections::VecDeque;

/// Reação de ciclo de vida de um elemento customizado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomElementReaction {
    /// O elemento foi conectado a um documento ativo (`connectedCallback`).
    Connected(NodeId),
    /// O elemento foi desconectado do documento ativo (`disconnectedCallback`).
    Disconnected(NodeId),
    /// Um atributo observado foi alterado ou removido (`attributeChangedCallback`).
    AttributeChanged {
        node: NodeId,
        name: SmolStr,
        old_value: Option<SmolStr>,
        new_value: Option<SmolStr>,
    },
    /// O elemento foi adotado por um novo documento (`adoptedCallback`).
    Adopted {
        node: NodeId,
        old_document: Option<String>,
        new_document: Option<String>,
    },
}

/// Pilha de reações de ciclo de vida de elementos customizados (`[CEReactions]`).
#[derive(Debug, Default)]
pub struct CustomElementReactionsStack {
    /// Pilha de quadros de elementos: cada nível armazena nós que tiveram reações enfileiradas.
    stack: Vec<Vec<NodeId>>,
    /// Fila de reações por `NodeId`.
    element_queues: FxHashMap<NodeId, VecDeque<CustomElementReaction>>,
    /// Fila plana para reações sem quadro de pilha explícito.
    flat_queue: VecDeque<CustomElementReaction>,
    /// Prevenção de reentrância recursiva descontrolada.
    is_processing: bool,
}

impl CustomElementReactionsStack {
    /// Cria uma nova pilha de reações vazia.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inicia um novo quadro de reações na pilha (`[CEReactions]` boundary).
    pub fn push_stack_frame(&mut self) {
        self.stack.push(Vec::new());
    }

    /// Encerra o quadro atual da pilha e retorna os nós afetados.
    pub fn pop_stack_frame(&mut self) -> Option<Vec<NodeId>> {
        self.stack.pop()
    }

    /// Enfileira uma reação de conexão (`connectedCallback`).
    pub fn enqueue_connected(&mut self, node: NodeId) {
        self.enqueue_reaction(CustomElementReaction::Connected(node));
    }

    /// Enfileira uma reação de desconexão (`disconnectedCallback`).
    pub fn enqueue_disconnected(&mut self, node: NodeId) {
        self.enqueue_reaction(CustomElementReaction::Disconnected(node));
    }

    /// Enfileira uma reação de alteração de atributo (`attributeChangedCallback`).
    pub fn enqueue_attribute_changed(
        &mut self,
        node: NodeId,
        name: impl Into<SmolStr>,
        old_value: Option<SmolStr>,
        new_value: Option<SmolStr>,
    ) {
        self.enqueue_reaction(CustomElementReaction::AttributeChanged {
            node,
            name: name.into(),
            old_value,
            new_value,
        });
    }

    /// Enfileira uma reação de adoção (`adoptedCallback`).
    pub fn enqueue_adopted(
        &mut self,
        node: NodeId,
        old_document: Option<String>,
        new_document: Option<String>,
    ) {
        self.enqueue_reaction(CustomElementReaction::Adopted {
            node,
            old_document,
            new_document,
        });
    }

    /// Enfileira uma reação genérica de ciclo de vida.
    pub fn enqueue_reaction(&mut self, reaction: CustomElementReaction) {
        let node_id = match &reaction {
            CustomElementReaction::Connected(id) => *id,
            CustomElementReaction::Disconnected(id) => *id,
            CustomElementReaction::AttributeChanged { node, .. } => *node,
            CustomElementReaction::Adopted { node, .. } => *node,
        };

        if let Some(top_frame) = self.stack.last_mut() {
            if !top_frame.contains(&node_id) {
                top_frame.push(node_id);
            }
            self.element_queues.entry(node_id).or_default().push_back(reaction);
        } else {
            self.flat_queue.push_back(reaction);
        }
    }

    /// Drena todas as reações pendentes em ordem FIFO.
    pub fn drain_all(&mut self) -> Vec<CustomElementReaction> {
        let mut result = Vec::new();
        result.extend(self.flat_queue.drain(..));
        for queue in self.element_queues.values_mut() {
            result.extend(queue.drain(..));
        }
        self.stack.clear();
        self.element_queues.clear();
        result
    }

    /// Retorna `true` se não houver reações pendentes.
    pub fn is_empty(&self) -> bool {
        self.flat_queue.is_empty()
            && self.element_queues.values().all(|q| q.is_empty())
    }

    /// Retorna o número total de reações pendentes.
    pub fn len(&self) -> usize {
        self.flat_queue.len()
            + self.element_queues.values().map(|q| q.len()).sum::<usize>()
    }

    /// Processa todas as reações pendentes na fila invocando os callbacks registrados no `CustomElementRegistry`.
    pub fn process_reactions(
        &mut self,
        arena: &Arena<crate::node::NodeData>,
        registry: &crate::custom_elements::CustomElementRegistry,
    ) {
        if self.is_processing {
            return;
        }
        self.is_processing = true;

        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10_000;

        while !self.is_empty() && iterations < MAX_ITERATIONS {
            iterations += 1;
            let reactions = self.drain_all();
            for reaction in reactions {
                match reaction {
                    CustomElementReaction::Connected(node_id) => {
                        if let Some(def) = get_definition_for_node(arena, registry, node_id) {
                            if let Some(ref cb) = def.callbacks.connected_callback {
                                cb(node_id);
                            }
                        }
                    }
                    CustomElementReaction::Disconnected(node_id) => {
                        if let Some(def) = get_definition_for_node(arena, registry, node_id) {
                            if let Some(ref cb) = def.callbacks.disconnected_callback {
                                cb(node_id);
                            }
                        }
                    }
                    CustomElementReaction::AttributeChanged {
                        node,
                        name,
                        old_value,
                        new_value,
                    } => {
                        if let Some(def) = get_definition_for_node(arena, registry, node) {
                            if let Some(ref cb) = def.callbacks.attribute_changed_callback {
                                cb(
                                    node,
                                    name.as_str(),
                                    old_value.as_deref(),
                                    new_value.as_deref(),
                                );
                            }
                        }
                    }
                    CustomElementReaction::Adopted {
                        node,
                        old_document,
                        new_document,
                    } => {
                        if let Some(def) = get_definition_for_node(arena, registry, node) {
                            if let Some(ref cb) = def.callbacks.adopted_callback {
                                cb(
                                    node,
                                    old_document.as_deref(),
                                    new_document.as_deref(),
                                );
                            }
                        }
                    }
                }
            }
        }

        self.is_processing = false;
    }
}

/// Helper para buscar a definição registrada de um nó customizado.
fn get_definition_for_node<'a>(
    arena: &'a Arena<crate::node::NodeData>,
    registry: &'a crate::custom_elements::CustomElementRegistry,
    node_id: NodeId,
) -> Option<&'a crate::custom_elements::CustomElementDefinition> {
    let aid = ArenaId::<crate::node::NodeData>::from_node_id(node_id)?;
    let node = arena.get(aid)?;
    let el = node.as_element()?;
    if el.custom_element_state() == crate::custom_elements::CustomElementState::Custom {
        if let Some(def_name) = el.custom_element_definition() {
            registry.get(def_name.as_str())
        } else {
            registry.get(el.tag_name.as_str())
        }
    } else {
        None
    }
}
