//! # Agendamento Ocioso e Cooperative Scheduling (W3C requestIdleCallback)
//!
//! Permite o agendamento de tarefas de baixa prioridade executadas exclusivamente
//! durante períodos ociosos (*Idle Periods*) entre quadros de renderização ou com timeout expirado.

use crate::id::TaskId;
use crate::time::Clock;
use crossbeam::queue::SegQueue;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Representa o limite temporal concedido a uma tarefa ociosa (`IdleDeadline`).
#[derive(Clone)]
pub struct IdleDeadline {
    deadline_ms: f64,
    clock: Arc<dyn Clock>,
    did_timeout: bool,
}

impl IdleDeadline {
    /// Cria uma nova instância de `IdleDeadline`.
    pub fn new(deadline_ms: f64, clock: Arc<dyn Clock>, did_timeout: bool) -> Self {
        Self {
            deadline_ms,
            clock,
            did_timeout,
        }
    }

    /// Retorna a estimativa em milissegundos do tempo restante no período ocioso atual.
    #[inline]
    pub fn time_remaining_ms(&self) -> f64 {
        let now = self.clock.now_highres();
        (self.deadline_ms - now).max(0.0)
    }

    /// Retorna `true` se a tarefa foi executada por expiração do timeout configurado,
    /// mesmo sem folga de tempo no período ocioso.
    #[inline]
    pub fn did_timeout(&self) -> bool {
        self.did_timeout
    }
}

pub type IdleTaskFn = Box<dyn FnOnce(IdleDeadline) + Send + 'static>;

/// Estrutura interna para uma tarefa ociosa agendada.
pub struct ScheduledIdleTask {
    pub id: TaskId,
    pub target_timeout_ms: Option<u64>,
    pub is_cancelled: Arc<AtomicBool>,
    pub func: IdleTaskFn,
}

/// Fila concorrente thread-safe para tarefas ociosas.
#[derive(Default)]
pub struct IdleTaskQueue {
    tasks: SegQueue<ScheduledIdleTask>,
}

impl IdleTaskQueue {
    /// Cria uma nova fila de tarefas ociosas.
    pub fn new() -> Self {
        Self {
            tasks: SegQueue::new(),
        }
    }

    /// Enfileira uma nova tarefa ociosa com timeout opcional (`requestIdleCallback`).
    pub fn post_idle_task<F>(&self, timeout: Option<Duration>, clock: &dyn Clock, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce(IdleDeadline) + Send + 'static,
    {
        let id = TaskId::new();
        let is_cancelled = Arc::new(AtomicBool::new(false));
        let target_timeout_ms = timeout.map(|d| clock.now_ms() + d.as_millis() as u64);

        self.tasks.push(ScheduledIdleTask {
            id,
            target_timeout_ms,
            is_cancelled: Arc::clone(&is_cancelled),
            func: Box::new(f),
        });

        (id, is_cancelled)
    }

    /// Processa tarefas ociosas disponíveis respeitando o orçamento de tempo do frame.
    pub fn process_idle_tasks(&self, clock: Arc<dyn Clock>, max_budget: Duration) -> usize {
        let start_time_ms = clock.now_highres();
        let deadline_ms = start_time_ms + max_budget.as_secs_f64() * 1000.0;
        let now_ms = clock.now_ms();
        let mut executed = 0;

        while let Some(task) = self.tasks.pop() {
            if task.is_cancelled.load(Ordering::Relaxed) {
                continue;
            }

            let timed_out = task
                .target_timeout_ms
                .map(|t| now_ms >= t)
                .unwrap_or(false);

            let current_time_ms = clock.now_highres();
            let has_time_remaining = current_time_ms < deadline_ms;

            if has_time_remaining || timed_out {
                let deadline = IdleDeadline::new(deadline_ms, Arc::clone(&clock), timed_out);
                (task.func)(deadline);
                executed += 1;
            } else {
                // Devolve a tarefa à fila se o orçamento de tempo esgotou e não houve timeout
                self.tasks.push(task);
                break;
            }
        }

        executed
    }

    /// Retorna `true` se não houver tarefas ociosas pendentes.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Retorna a quantidade aproximada de tarefas ociosas pendentes.
    #[inline]
    pub fn len(&self) -> usize {
        self.tasks.len()
    }
}
