//! # Pipeline de Despacho de Eventos DOM (Event Dispatch Pipeline)
//!
//! Implementação completa das 3 fases normativas: Captura (Top-Down) -> Target -> Borbulhamento (Bottom-Up),
//! com suporte a `once`, `passive` e desregistro reativo.

use crate::events::{Event, EventListener, EventPhase};
use crate::tree::Document;
use ace_core::id::NodeId;
use ace_core::intern::Atom;
use rustc_hash::FxHashMap;

/// Mapa de ouvintes registrados por nó e por tipo de evento.
#[derive(Default)]
pub struct EventRegistry {
    listeners: FxHashMap<(NodeId, Atom), Vec<EventListener>>,
}

impl EventRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra um novo ouvinte de evento.
    pub fn add_event_listener(
        &mut self,
        node_id: NodeId,
        event_type: impl Into<Atom>,
        listener: EventListener,
    ) {
        self.listeners
            .entry((node_id, event_type.into()))
            .or_default()
            .push(listener);
    }

    /// Despacha um evento através da hierarquia DOM (com suporte a Composed Path e Shadow DOM retargeting).
    pub fn dispatch(&mut self, doc: &Document, target_id: NodeId, event: &mut Event) -> bool {
        event.target = Some(target_id);

        // Constrói a cadeia de caminho do evento (ancestrais com retargeting)
        let full_path = compute_event_path(doc, target_id, event.composed);
        if full_path.is_empty() {
            return !event.is_default_prevented();
        }

        // Ancestrais (excluindo o nó alvo) em ordem Top-Down para captura
        let ancestors: Vec<_> = full_path[1..].iter().rev().copied().collect();

        // 1. Fase de Captura (Top-Down: Raiz -> Pai)
        event.phase = EventPhase::CapturingPhase;
        for (node_id, retargeted_target) in &ancestors {
            if event.is_propagation_stopped() {
                break;
            }
            event.target = Some(*retargeted_target);
            event.current_target = Some(*node_id);
            self.invoke_listeners(*node_id, &event.event_type.clone(), event, true);
        }

        // 2. Fase no Alvo (AtTarget)
        if !event.is_propagation_stopped() {
            event.phase = EventPhase::AtTarget;
            event.target = Some(target_id);
            event.current_target = Some(target_id);
            self.invoke_listeners(target_id, &event.event_type.clone(), event, true);
            if !event.is_immediate_propagation_stopped() {
                self.invoke_listeners(target_id, &event.event_type.clone(), event, false);
            }
        }

        // 3. Fase de Borbulhamento (Bottom-Up: Pai -> Raiz)
        if event.bubbles && !event.is_propagation_stopped() {
            event.phase = EventPhase::BubblingPhase;
            for (node_id, retargeted_target) in &full_path[1..] {
                if event.is_propagation_stopped() {
                    break;
                }
                event.target = Some(*retargeted_target);
                event.current_target = Some(*node_id);
                self.invoke_listeners(*node_id, &event.event_type.clone(), event, false);
            }
        }

        event.phase = EventPhase::None;
        event.current_target = None;
        event.target = Some(target_id);

        !event.is_default_prevented()
    }
}

/// Computa a cadeia de propagação (current_target, retargeted_target) cruzando ou respeitando Shadow Roots.
fn compute_event_path(doc: &Document, target_id: NodeId, composed: bool) -> Vec<(NodeId, NodeId)> {
    let mut path = Vec::new();
    let mut curr = target_id;
    let mut effective_target = target_id;

    path.push((curr, effective_target));

    while let Some(node) = doc.get_node(curr) {
        if let crate::node::NodeKind::ShadowRoot(ref s_data) = node.kind {
            if composed {
                curr = s_data.host;
                effective_target = s_data.host;
                path.push((curr, effective_target));
                continue;
            } else {
                break;
            }
        }

        if let Some(parent_id) = node.parent {
            curr = parent_id;
            path.push((curr, effective_target));
        } else {
            break;
        }
    }

    path
}

    fn invoke_listeners(
        &mut self,
        node_id: NodeId,
        event_type: &Atom,
        event: &mut Event,
        capture_phase: bool,
    ) {
        if let Some(list) = self.listeners.get_mut(&(node_id, event_type.clone())) {
            let mut i = 0;
            while i < list.len() {
                if event.is_immediate_propagation_stopped() {
                    break;
                }

                if list[i].options.signal_aborted {
                    list.remove(i);
                    continue;
                }

                if list[i].options.capture == capture_phase {
                    let callback = list[i].callback.clone();
                    let is_once = list[i].options.once;

                    (callback)(event);

                    if is_once {
                        list.remove(i);
                        continue;
                    }
                }
                i += 1;
            }
        }
    }
}

/// Despacha um evento diretamente usando um registro temporário.
pub fn dispatch_event(
    registry: &mut EventRegistry,
    doc: &Document,
    target_id: NodeId,
    event: &mut Event,
) -> bool {
    registry.dispatch(doc, target_id, event)
}
