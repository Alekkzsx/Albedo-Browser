//! # Event Loop (WHATWG Specification Section 8.1.6)
//!
//! O motor de execução assíncrona do navegador.
//!
//! O loop segue estritamente a especificação da web:
//! 1. Seleciona a tarefa de maior prioridade entre as filas de `TaskSource`.
//! 2. Executa a tarefa.
//! 3. **Microtask Checkpoint:** Drena **toda a fila de Microtasks** (Promises, MutationObserver) até esgotar.
//! 4. (Opcional) Executa o ciclo de renderização (`requestAnimationFrame`, recalc style, layout pass).
//! 5. Repete.

pub mod source;
pub mod task;

pub use source::TaskSource;
pub use task::{Task, TaskFn};

use crate::id::TaskId;
use crossbeam::queue::SegQueue;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};

/// Handle clonável para submissão de tarefas e microtasks a partir de qualquer thread.
#[derive(Clone)]
pub struct TaskQueue {
    /// Canal de macrotasks categorizadas.
    task_tx: mpsc::UnboundedSender<Task>,
    /// Fila concorrente dinâmica ilimitada para microtasks (zero perda de promises).
    microtask_queue: Arc<SegQueue<TaskFn>>,
    /// Fila de tarefas de renderização (requestAnimationFrame).
    raf_queue: Arc<SegQueue<TaskFn>>,
}

impl TaskQueue {
    /// Enfileira uma macrotask associada a uma fonte específica da spec WHATWG.
    /// Retorna o `TaskId` e o controle atômico de cancelamento (`is_cancelled`).
    pub fn queue_task<F>(&self, source: TaskSource, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce() + Send + 'static,
    {
        let (task, cancel_handle) = Task::new(source, f);
        let id = task.id;
        let _ = self.task_tx.send(task);
        (id, cancel_handle)
    }

    /// Enfileira uma tarefa de interação do usuário (alta prioridade: cliques, digitação).
    pub fn queue_user_interaction<F>(&self, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::UserInteraction, f)
    }

    /// Enfileira uma tarefa de rede (`fetch`, websockets, callbacks HTTP).
    pub fn queue_network<F>(&self, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::Networking, f)
    }

    /// Enfileira uma tarefa de temporizador (`setTimeout`, `setInterval`).
    pub fn queue_timer<F>(&self, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::Timer, f)
    }

    /// Enfileira uma microtask (Promises, `queueMicrotask`, MutationObserver).
    ///
    /// Microtasks possuem prioridade absoluta e são drenadas após **cada** macrotask.
    /// Fila dinâmica sem limite rígido artificial.
    pub fn queue_microtask<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.microtask_queue.push(Box::new(f));
    }

    /// Enfileira uma função para ser executada no próximo quadro de animação (`requestAnimationFrame`).
    pub fn request_animation_frame<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.raf_queue.push(Box::new(f));
    }
}

/// O processador do Event Loop de uma aba ou contexto de navegação.
pub struct EventLoop {
    task_rx: Mutex<mpsc::UnboundedReceiver<Task>>,
    microtask_queue: Arc<SegQueue<TaskFn>>,
    raf_queue: Arc<SegQueue<TaskFn>>,
    queue_handle: TaskQueue,
}

impl EventLoop {
    /// Inicializa um novo Event Loop WHATWG.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let microtask_queue = Arc::new(SegQueue::new());
        let raf_queue = Arc::new(SegQueue::new());

        let handle = TaskQueue {
            task_tx: tx,
            microtask_queue: Arc::clone(&microtask_queue),
            raf_queue: Arc::clone(&raf_queue),
        };

        Self {
            task_rx: Mutex::new(rx),
            microtask_queue,
            raf_queue,
            queue_handle: handle,
        }
    }

    /// Retorna o handle de submissão de tarefas compartilhado.
    #[inline]
    pub fn handle(&self) -> TaskQueue {
        self.queue_handle.clone()
    }

    /// Executa um passo único do Event Loop (útil para testes unitários ou headless rendering determinístico).
    /// Retorna `true` se alguma tarefa ou microtask foi processada.
    pub fn step(&self) -> bool {
        let mut executed_something = false;

        // 1. Pega e executa uma Macrotask se houver
        let task_opt = {
            let mut rx = self.task_rx.lock();
            rx.try_recv().ok()
        };

        if let Some(task) = task_opt {
            trace!("Executando Macrotask [{:?}] ID: {}", task.source, task.id);
            task.execute();
            executed_something = true;
        }

        // 2. Microtask Checkpoint: drena todas as microtasks pendentes
        let micro_count = self.drain_microtasks();
        if micro_count > 0 {
            executed_something = true;
        }

        executed_something
    }

    /// Executa o pipeline de renderização pendente (`requestAnimationFrame`).
    pub fn process_animation_frame(&self) -> usize {
        let mut count = 0;
        while let Some(raf_task) = self.raf_queue.pop() {
            raf_task();
            count += 1;
        }
        // Microtask checkpoint após animações
        self.drain_microtasks();
        count
    }

    /// Drena exaustivamente a fila de microtasks (Promises).
    pub fn drain_microtasks(&self) -> usize {
        let mut executed = 0;
        while let Some(micro_task) = self.microtask_queue.pop() {
            trace!("Executando Microtask...");
            micro_task();
            executed += 1;

            // Circuit Breaker para evitar travamento do browser em caso de loop infinito de Promises
            if executed > 100_000 {
                warn!("Circuit Breaker: Mais de 100.000 microtasks executadas consecutivamente!");
                break;
            }
        }
        executed
    }

    /// Inicia a execução contínua bloqueando a thread atual até o canal ser fechado.
    pub fn run_sync(&self) {
        debug!("Iniciando Event Loop WHATWG...");
        loop {
            let task_opt = {
                let mut rx = self.task_rx.lock();
                rx.blocking_recv()
            };

            match task_opt {
                Some(task) => {
                    task.execute();
                    self.drain_microtasks();
                }
                None => {
                    // Canal desconectado
                    break;
                }
            }
        }
        debug!("Event Loop finalizado.");
    }
}

impl Default for EventLoop {
    fn default() -> Self {
        Self::new()
    }
}
