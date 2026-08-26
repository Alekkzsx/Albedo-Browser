//! # Registro Global de Live Ranges (WHATWG DOM §5.5)
//!
//! Rastreia e atualiza automaticamente os pontos de contorno de todos os `Range`s ativos
//! durante mutações estruturais na árvore DOM (`split_text`, `remove_child`, `insert_before`).

use crate::range::Range;
use ace_core::id::NodeId;
use std::sync::{Arc, Mutex, Weak};

/// Identificador opaco para um Live Range rastreado no documento.
#[derive(Clone)]
pub struct LiveRangeHandle {
    range: Arc<Mutex<Range>>,
}

impl LiveRangeHandle {
    /// Cria um novo handle encapsulando um Range.
    pub fn new(range: Range) -> Self {
        Self {
            range: Arc::new(Mutex::new(range)),
        }
    }

    /// Obtém uma cópia do estado atual do Range.
    pub fn get_range(&self) -> Range {
        self.range.lock().unwrap().clone()
    }

    /// Executa uma mutação no Range.
    pub fn with_range_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Range) -> R,
    {
        let mut r = self.range.lock().unwrap();
        f(&mut r)
    }
}

/// Registro central de Live Ranges de um Documento.
#[derive(Debug, Default)]
pub struct LiveRangeRegistry {
    ranges: Vec<Weak<Mutex<Range>>>,
}

impl LiveRangeRegistry {
    /// Cria um novo registro vazio.
    pub fn new() -> Self {
        Self {
            ranges: Vec::new(),
        }
    }

    /// Registra um Range para rastreamento contínuo de mutações.
    pub fn register(&mut self, handle: &LiveRangeHandle) {
        self.cleanup();
        self.ranges.push(Arc::downgrade(&handle.range));
    }

    /// Remove referências mortas (Weak pointers desalocados).
    fn cleanup(&mut self) {
        self.ranges.retain(|weak| weak.strong_count() > 0);
    }

    /// Notifica o registro sobre uma operação de divisão de texto (`split_text`).
    pub fn notify_split_text(&mut self, orig_node: NodeId, new_node: NodeId, split_offset: usize) {
        self.cleanup();
        for weak in &self.ranges {
            if let Some(arc) = weak.upgrade() {
                if let Ok(mut r) = arc.lock() {
                    r.adjust_for_split_text(orig_node, new_node, split_offset);
                }
            }
        }
    }

    /// Notifica o registro sobre a remoção de um nó filho (`remove_child`).
    pub fn notify_node_removal(&mut self, removed_node: NodeId, parent_id: NodeId, child_index: usize) {
        self.cleanup();
        for weak in &self.ranges {
            if let Some(arc) = weak.upgrade() {
                if let Ok(mut r) = arc.lock() {
                    r.adjust_for_node_removal(removed_node, parent_id, child_index);
                }
            }
        }
    }

    /// Notifica o registro sobre a inserção de um nó filho (`insert_before` / `append_child`).
    pub fn notify_node_insertion(&mut self, parent_id: NodeId, insertion_index: usize) {
        self.cleanup();
        for weak in &self.ranges {
            if let Some(arc) = weak.upgrade() {
                if let Ok(mut r) = arc.lock() {
                    r.adjust_for_node_insertion(parent_id, insertion_index);
                }
            }
        }
    }
}
