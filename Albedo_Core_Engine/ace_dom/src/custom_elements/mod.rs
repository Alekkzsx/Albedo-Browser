//! # Web Components & Custom Elements (WHATWG HTML §4.13)
//!
//! Fornece o registro de componentes web e fila de reações de ciclo de vida.

pub mod registry;

pub use registry::{is_valid_custom_element_name, CustomElementDefinition, CustomElementRegistry};

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
}
