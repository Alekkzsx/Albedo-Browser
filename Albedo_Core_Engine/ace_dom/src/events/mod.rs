//! # Modelo de Eventos DOM Nível 3 (WHATWG DOM Event Architecture)
//!
//! Estruturas de eventos, fases de propagação e despacho de eventos.

pub mod dispatch;

pub use dispatch::dispatch_event;

use ace_core::id::NodeId;
use ace_core::intern::Atom;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Fases do ciclo de vida de um evento DOM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EventPhase {
    #[default]
    None = 0,
    CapturingPhase = 1,
    AtTarget = 2,
    BubblingPhase = 3,
}

/// Um evento DOM instanciado para propagação na árvore.
#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: Atom,
    pub target: Option<NodeId>,
    pub current_target: Option<NodeId>,
    pub phase: EventPhase,
    pub bubbles: bool,
    pub cancelable: bool,
    propagation_stopped: Arc<AtomicBool>,
    immediate_propagation_stopped: Arc<AtomicBool>,
    default_prevented: Arc<AtomicBool>,
}

impl Event {
    /// Cria um novo evento configurado.
    pub fn new(event_type: impl Into<Atom>, bubbles: bool, cancelable: bool) -> Self {
        Self {
            event_type: event_type.into(),
            target: None,
            current_target: None,
            phase: EventPhase::None,
            bubbles,
            cancelable,
            propagation_stopped: Arc::new(AtomicBool::new(false)),
            immediate_propagation_stopped: Arc::new(AtomicBool::new(false)),
            default_prevented: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Interrompe a propagação do evento para os próximos nós na cadeia.
    #[inline]
    pub fn stop_propagation(&self) {
        self.propagation_stopped.store(true, Ordering::Release);
    }

    /// Interrompe a propagação imediatamente, inclusive para outros ouvintes no mesmo nó.
    #[inline]
    pub fn stop_immediate_propagation(&self) {
        self.propagation_stopped.store(true, Ordering::Release);
        self.immediate_propagation_stopped.store(true, Ordering::Release);
    }

    /// Cancela a ação padrão associada ao evento se `cancelable == true`.
    #[inline]
    pub fn prevent_default(&self) {
        if self.cancelable {
            self.default_prevented.store(true, Ordering::Release);
        }
    }

    #[inline]
    pub fn is_propagation_stopped(&self) -> bool {
        self.propagation_stopped.load(Ordering::Acquire)
    }

    #[inline]
    pub fn is_immediate_propagation_stopped(&self) -> bool {
        self.immediate_propagation_stopped.load(Ordering::Acquire)
    }

    #[inline]
    pub fn is_default_prevented(&self) -> bool {
        self.default_prevented.load(Ordering::Acquire)
    }
}

/// Ouvinte de evento com opções de captura e execução única.
pub struct EventListener {
    pub callback: Box<dyn Fn(&mut Event) + Send + Sync>,
    pub capture: bool,
    pub once: bool,
    pub passive: bool,
}

impl std::fmt::Debug for EventListener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventListener")
            .field("capture", &self.capture)
            .field("once", &self.once)
            .field("passive", &self.passive)
            .finish()
    }
}
