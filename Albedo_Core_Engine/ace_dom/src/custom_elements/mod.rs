//! # Web Components & Custom Elements (WHATWG HTML §4.13)
//!
//! Fornece o registro central de componentes customizados (`CustomElementRegistry`),
//! validação estrita PCENChar, callbacks de ciclo de vida e pilha de reações (`[CEReactions]`).

pub mod reaction_stack;
pub mod registry;

pub use reaction_stack::{CustomElementReaction, CustomElementReactionsStack};
pub use registry::{
    is_pcen_char, is_valid_custom_element_name, CustomElementCallbacks, CustomElementDefinition,
    CustomElementRegistry, CustomElementState, ElementDefinitionOptions,
};

use ace_core::id::NodeId;
use ace_core::intern::Atom;
use smol_str::SmolStr;
use std::collections::VecDeque;

/// Reação de ciclo de vida de um elemento customizado pronta para entrega.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleReaction {
    /// O elemento foi conectado a um documento ativo (`connectedCallback`).
    Connected(NodeId),
    /// O elemento foi desconectado do documento (`disconnectedCallback`).
    Disconnected(NodeId),
    /// Um atributo observado foi alterado (`attributeChangedCallback`).
    AttributeChanged {
        node: NodeId,
        name: Atom,
        old_value: Option<SmolStr>,
        new_value: Option<SmolStr>,
    },
    /// O elemento foi adotado por um novo documento (`adoptedCallback`).
    Adopted(NodeId),
}

impl From<CustomElementReaction> for LifecycleReaction {
    fn from(r: CustomElementReaction) -> Self {
        match r {
            CustomElementReaction::Connected(id) => LifecycleReaction::Connected(id),
            CustomElementReaction::Disconnected(id) => LifecycleReaction::Disconnected(id),
            CustomElementReaction::AttributeChanged {
                node,
                name,
                old_value,
                new_value,
            } => LifecycleReaction::AttributeChanged {
                node,
                name: Atom::new(name.as_str()),
                old_value,
                new_value,
            },
            CustomElementReaction::Adopted { node, .. } => LifecycleReaction::Adopted(node),
        }
    }
}

impl From<LifecycleReaction> for CustomElementReaction {
    fn from(r: LifecycleReaction) -> Self {
        match r {
            LifecycleReaction::Connected(id) => CustomElementReaction::Connected(id),
            LifecycleReaction::Disconnected(id) => CustomElementReaction::Disconnected(id),
            LifecycleReaction::AttributeChanged {
                node,
                name,
                old_value,
                new_value,
            } => CustomElementReaction::AttributeChanged {
                node,
                name: SmolStr::new(name.as_str()),
                old_value,
                new_value,
            },
            LifecycleReaction::Adopted(id) => CustomElementReaction::Adopted {
                node: id,
                old_document: None,
                new_document: None,
            },
        }
    }
}

/// Fila de reações de ciclo de vida de elementos customizados.
#[derive(Debug, Default)]
pub struct LifecycleQueue {
    queue: VecDeque<LifecycleReaction>,
}

impl LifecycleQueue {
    /// Cria uma nova fila de ciclo de vida.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Enfileira uma reação de ciclo de vida.
    pub fn enqueue(&mut self, reaction: LifecycleReaction) {
        self.queue.push_back(reaction);
    }

    /// Drena e retorna todas as reações pendentes.
    pub fn drain(&mut self) -> Vec<LifecycleReaction> {
        self.queue.drain(..).collect()
    }

    /// Retorna `true` se a fila estiver vazia.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Retorna a contagem de reações pendentes.
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}
