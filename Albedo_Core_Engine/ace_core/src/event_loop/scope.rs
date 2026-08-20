//! # Escopos de Tarefas e Cancelamento em Lote (Chromium WeakPtrFactory Pattern)
//!
//! Vincula o ciclo de vida de tarefas enfileiradas a uma aba, frame ou contexto de documento,
//! garantindo cancelamento automático atômico quando o escopo é descartado.

use crate::event_loop::source::TaskSource;
use crate::event_loop::TaskQueue;
use crate::id::TaskId;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug)]
struct TaskScopeInner {
    is_active: AtomicBool,
    owner_count: AtomicU32,
}

/// Gerenciador de ciclo de vida de tarefas associadas a uma entidade (Aba, Documento, Frame).
///
/// Ao ser destruído (`Drop`), invalida atomicamente todas as tarefas agendadas sob seu controle.
#[derive(Debug)]
pub struct TaskScope {
    inner: Arc<TaskScopeInner>,
}

impl TaskScope {
    /// Cria um novo escopo ativo de tarefas.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(TaskScopeInner {
                is_active: AtomicBool::new(true),
                owner_count: AtomicU32::new(1),
            }),
        }
    }

    /// Retorna `true` se o escopo ainda estiver ativo e válido.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.inner.is_active.load(Ordering::Acquire)
    }

    /// Invalida o escopo imediatamente, cancelando todas as tarefas pendentes vinculadas.
    pub fn invalidate(&self) {
        self.inner.is_active.store(false, Ordering::Release);
    }

    /// Cria uma fila de tarefas encapsulada vinculada a este escopo.
    pub fn wrap_queue(&self, queue: TaskQueue) -> ScopedTaskQueue {
        ScopedTaskQueue {
            scope: Arc::clone(&self.inner),
            queue,
        }
    }
}

impl Clone for TaskScope {
    fn clone(&self) -> Self {
        self.inner.owner_count.fetch_add(1, Ordering::Relaxed);
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Default for TaskScope {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TaskScope {
    fn drop(&mut self) {
        if self.inner.owner_count.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.invalidate();
        }
    }
}

/// Fila de tarefas associada a um `TaskScope`, interceptando chamadas e descartando tarefas de escopos invalidados.
#[derive(Clone)]
pub struct ScopedTaskQueue {
    scope: Arc<TaskScopeInner>,
    queue: TaskQueue,
}

impl ScopedTaskQueue {
    /// Enfileira uma macrotask associada ao escopo. Retorna `Some(TaskId)` se o escopo estiver ativo.
    pub fn queue_task<F>(&self, source: TaskSource, f: F) -> Option<TaskId>
    where
        F: FnOnce() + Send + 'static,
    {
        if !self.scope.is_active.load(Ordering::Acquire) {
            return None;
        }

        let scope_flag = Arc::clone(&self.scope);
        let (id, _cancel) = self.queue.queue_task(source, move || {
            if scope_flag.is_active.load(Ordering::Acquire) {
                f();
            }
        });

        Some(id)
    }

    /// Enfileira uma tarefa de interação do usuário com proteção de escopo.
    pub fn queue_user_interaction<F>(&self, f: F) -> Option<TaskId>
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::UserInteraction, f)
    }

    /// Enfileira uma tarefa de manipulação de DOM com proteção de escopo.
    pub fn queue_dom<F>(&self, f: F) -> Option<TaskId>
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::DomManipulation, f)
    }

    /// Enfileira uma tarefa de rede com proteção de escopo.
    pub fn queue_network<F>(&self, f: F) -> Option<TaskId>
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::Networking, f)
    }

    /// Enfileira uma tarefa de navegação e histórico com proteção de escopo.
    pub fn queue_history<F>(&self, f: F) -> Option<TaskId>
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::HistoryTraversal, f)
    }

    /// Enfileira uma tarefa de renderização com proteção de escopo.
    pub fn queue_rendering<F>(&self, f: F) -> Option<TaskId>
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::Rendering, f)
    }

    /// Enfileira uma tarefa interna do motor com proteção de escopo.
    pub fn queue_internal<F>(&self, f: F) -> Option<TaskId>
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::Internal, f)
    }

    /// Enfileira uma tarefa de temporizador imediato com proteção de escopo.
    pub fn queue_timer<F>(&self, f: F) -> Option<TaskId>
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::Timer, f)
    }

    /// Agenda um temporizador (`setTimeout`) protegido por escopo.
    pub fn schedule_timer<F>(&self, delay: Duration, f: F) -> Option<TaskId>
    where
        F: FnOnce() + Send + 'static,
    {
        if !self.scope.is_active.load(Ordering::Acquire) {
            return None;
        }

        let scope_flag = Arc::clone(&self.scope);
        let (id, _cancel) = self.queue.schedule_timer(delay, move || {
            if scope_flag.is_active.load(Ordering::Acquire) {
                f();
            }
        });

        Some(id)
    }

    /// Enfileira uma microtask protegida por escopo.
    pub fn queue_microtask<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if !self.scope.is_active.load(Ordering::Acquire) {
            return;
        }

        let scope_flag = Arc::clone(&self.scope);
        self.queue.queue_microtask(move || {
            if scope_flag.is_active.load(Ordering::Acquire) {
                f();
            }
        });
    }

    /// Enfileira uma animação (`requestAnimationFrame`) protegida por escopo.
    pub fn request_animation_frame<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if !self.scope.is_active.load(Ordering::Acquire) {
            return;
        }

        let scope_flag = Arc::clone(&self.scope);
        self.queue.request_animation_frame(move || {
            if scope_flag.is_active.load(Ordering::Acquire) {
                f();
            }
        });
    }

    /// Retorna a fila base sem encapsulamento de escopo.
    #[inline]
    pub fn raw_queue(&self) -> &TaskQueue {
        &self.queue
    }
}
