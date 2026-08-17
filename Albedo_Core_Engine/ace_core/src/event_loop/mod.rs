//! # Event Loop (WHATWG Specification Section 8.1.6)
//!
//! O motor de execução assíncrona do navegador.
//!
//! O loop segue estritamente a especificação da web:
//! 1. Processa temporizadores expirados de acordo com o relógio (`Clock`).
//! 2. Seleciona e executa uma tarefa entre as filas de `TaskSource`.
//! 3. **Microtask Checkpoint:** Drena **toda a fila de Microtasks** (Promises, MutationObserver) até esgotar.
//! 4. (Opcional) Executa o ciclo de renderização (`requestAnimationFrame`, recalc style, layout pass).
//! 5. Repete.

pub mod coalescer;
pub mod source;
pub mod task;
pub mod utils;

pub use coalescer::{CoalescedMovement, InputEventCoalescer};
pub use source::TaskSource;
pub use task::{Task, TaskFn};
pub use utils::{compute_deadline, duration_to_ms, fps_to_interval, is_deadline_passed, ms_to_duration};

use crate::error::AceError;
use crate::id::TaskId;
use crate::time::{Clock, MonotonicClock};
use crossbeam::queue::SegQueue;
use parking_lot::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};

/// Representa um temporizador agendado (`setTimeout` / `setInterval`) com prazo em milissegundos.
struct ScheduledTimer {
    #[allow(dead_code)]
    id: TaskId,
    target_ms: u64,
    is_cancelled: Arc<AtomicBool>,
    func: TaskFn,
}

/// Handle clonável para submissão de tarefas e microtasks a partir de qualquer thread.
#[derive(Clone)]
pub struct TaskQueue {
    /// Canal de macrotasks categorizadas.
    task_tx: mpsc::UnboundedSender<Task>,
    /// Fila concorrente dinâmica ilimitada para microtasks (zero perda de promises).
    microtask_queue: Arc<SegQueue<TaskFn>>,
    /// Fila de tarefas de renderização (requestAnimationFrame).
    raf_queue: Arc<SegQueue<TaskFn>>,
    /// Lista de temporizadores agendados.
    timers: Arc<Mutex<Vec<ScheduledTimer>>>,
    /// Relógio de referência para resolução de prazos.
    clock: Arc<dyn Clock>,
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

    /// Enfileira uma tarefa de temporizador imediato.
    pub fn queue_timer<F>(&self, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::Timer, f)
    }

    /// Agenda um temporizador para disparar após um atraso (`delay`) no Event Loop (`setTimeout`).
    /// Utiliza o `Clock` configurado para suportar relógios determinísticos (`MockClock`) em testes.
    pub fn schedule_timer<F>(&self, delay: Duration, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce() + Send + 'static,
    {
        let id = TaskId::new();
        let is_cancelled = Arc::new(AtomicBool::new(false));
        let target_ms = self.clock.now_ms() + delay.as_millis() as u64;

        let timer = ScheduledTimer {
            id,
            target_ms,
            is_cancelled: Arc::clone(&is_cancelled),
            func: Box::new(f),
        };

        self.timers.lock().push(timer);
        (id, is_cancelled)
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

    /// Despacha uma tarefa intensiva de CPU para execução paralela resiliente (`spawn_safe`)
    /// e encaminha o resultado de volta como uma macrotask no Event Loop.
    pub fn spawn_cpu_task<F, R, C>(&self, cpu_work: F, on_complete: C)
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
        C: FnOnce(Result<R, AceError>) + Send + 'static,
    {
        let queue = self.clone();
        rayon::spawn(move || {
            let result = crate::task::spawn_safe(cpu_work);
            queue.queue_task(TaskSource::Internal, move || {
                on_complete(result);
            });
        });
    }
}

/// O processador do Event Loop de uma aba ou contexto de navegação.
pub struct EventLoop {
    task_rx: Mutex<mpsc::UnboundedReceiver<Task>>,
    microtask_queue: Arc<SegQueue<TaskFn>>,
    raf_queue: Arc<SegQueue<TaskFn>>,
    timers: Arc<Mutex<Vec<ScheduledTimer>>>,
    clock: Arc<dyn Clock>,
    queue_handle: TaskQueue,
}

impl EventLoop {
    /// Inicializa um novo Event Loop com o relógio monotônico padrão do sistema.
    pub fn new() -> Self {
        Self::with_clock(Arc::new(MonotonicClock::new()))
    }

    /// Inicializa um Event Loop acoplado a uma implementação customizada de `Clock` (ex: `MockClock` para testes).
    pub fn with_clock(clock: Arc<dyn Clock>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let microtask_queue = Arc::new(SegQueue::new());
        let raf_queue = Arc::new(SegQueue::new());
        let timers = Arc::new(Mutex::new(Vec::new()));

        let handle = TaskQueue {
            task_tx: tx,
            microtask_queue: Arc::clone(&microtask_queue),
            raf_queue: Arc::clone(&raf_queue),
            timers: Arc::clone(&timers),
            clock: Arc::clone(&clock),
        };

        Self {
            task_rx: Mutex::new(rx),
            microtask_queue,
            raf_queue,
            timers,
            clock,
            queue_handle: handle,
        }
    }

    /// Retorna o relógio utilizado por este Event Loop.
    #[inline]
    pub fn clock(&self) -> &dyn Clock {
        &*self.clock
    }

    /// Retorna o handle de submissão de tarefas compartilhado.
    #[inline]
    pub fn handle(&self) -> TaskQueue {
        self.queue_handle.clone()
    }

    /// Processa todos os temporizadores agendados cujo prazo já expirou.
    pub fn process_expired_timers(&self) -> usize {
        let now = self.clock.now_ms();
        let mut expired = Vec::new();

        {
            let mut timers = self.timers.lock();
            let mut i = 0;
            while i < timers.len() {
                if timers[i].target_ms <= now {
                    expired.push(timers.remove(i));
                } else {
                    i += 1;
                }
            }
        }

        let count = expired.len();
        for timer in expired {
            if !timer.is_cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                (timer.func)();
            }
        }

        count
    }

    /// Executa um passo único do Event Loop (útil para testes unitários ou headless rendering determinístico).
    /// Retorna `true` se alguma tarefa, timer ou microtask foi processada.
    pub fn step(&self) -> bool {
        let mut executed_something = false;

        // 1. Processa temporizadores cujo prazo expirou
        let expired_count = self.process_expired_timers();
        if expired_count > 0 {
            executed_something = true;
        }

        // 2. Pega e executa uma Macrotask se houver
        let task_opt = {
            let mut rx = self.task_rx.lock();
            rx.try_recv().ok()
        };

        if let Some(task) = task_opt {
            trace!("Executando Macrotask [{:?}] ID: {}", task.source, task.id);
            task.execute();
            executed_something = true;
        }

        // 3. Microtask Checkpoint: drena todas as microtasks pendentes
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
            self.process_expired_timers();

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
