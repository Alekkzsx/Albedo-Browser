//! # Tarefas e Envelopes de Execução
//!
//! Encapsulamento de tarefas do Event Loop com identificação única, fonte e cancelamento atômico.

use super::source::TaskSource;
use crate::id::TaskId;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub type TaskFn = Box<dyn FnOnce() + Send + 'static>;

/// Representa uma tarefa enfileirada no Event Loop.
pub struct Task {
    /// Identificador único da tarefa.
    pub id: TaskId,
    /// Fonte da especificação WHATWG.
    pub source: TaskSource,
    /// Flag atômica de cancelamento (usada para `clearTimeout` / `clearInterval`).
    pub is_cancelled: Arc<AtomicBool>,
    /// A closure a ser executada.
    pub func: TaskFn,
}

impl Task {
    /// Cria uma nova tarefa ativa.
    pub fn new<F>(source: TaskSource, func: F) -> (Self, Arc<AtomicBool>)
    where
        F: FnOnce() + Send + 'static,
    {
        let id = TaskId::new();
        let is_cancelled = Arc::new(AtomicBool::new(false));
        (
            Self {
                id,
                source,
                is_cancelled: Arc::clone(&is_cancelled),
                func: Box::new(func),
            },
            is_cancelled,
        )
    }

    /// Executa a tarefa se ela não tiver sido cancelada.
    #[inline]
    pub fn execute(self) -> bool {
        if !self.is_cancelled.load(Ordering::Relaxed) {
            (self.func)();
            true
        } else {
            false
        }
    }
}
