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
pub mod idle;
pub mod scope;
pub mod source;
pub mod task;
pub mod utils;

pub use coalescer::{CoalescedMovement, InputEventCoalescer};
pub use idle::{IdleDeadline, IdleTaskFn, IdleTaskQueue, ScheduledIdleTask};
pub use scope::{ScopedTaskQueue, TaskScope};
pub use source::TaskSource;
pub use task::{Task, TaskFn};
pub use utils::{
    compute_deadline, duration_to_ms, fps_to_interval, is_deadline_passed, ms_to_duration,
};

use crate::error::AceError;
use crate::id::TaskId;
use crate::time::{Clock, MonotonicClock};
use crossbeam::queue::SegQueue;
use parking_lot::Mutex;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, trace, warn};

/// Número total de fontes de tarefas padronizadas no WHATWG HTML.
const NUM_TASK_SOURCES: usize = 7;

/// Representa um temporizador agendado (`setTimeout` / `setInterval`) ordenado por prazo.
struct ScheduledTimer {
    id: TaskId,
    target_ms: u64,
    #[allow(dead_code)]
    nesting_level: usize,
    is_cancelled: Arc<AtomicBool>,
    func: TaskFn,
}

impl PartialEq for ScheduledTimer {
    fn eq(&self, other: &Self) -> bool {
        self.target_ms == other.target_ms && self.id == other.id
    }
}

impl Eq for ScheduledTimer {}

impl PartialOrd for ScheduledTimer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledTimer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Min-Heap: menor target_ms tem maior prioridade de desempilhamento
        other
            .target_ms
            .cmp(&self.target_ms)
            .then_with(|| other.id.raw().cmp(&self.id.raw()))
    }
}

/// Conjunto de filas concorrentes dedicadas por fonte de tarefas WHATWG.
struct TaskSourceQueues {
    queues: [SegQueue<Task>; NUM_TASK_SOURCES],
}

impl TaskSourceQueues {
    fn new() -> Self {
        Self {
            queues: [
                SegQueue::new(), // UserInteraction (0)
                SegQueue::new(), // DomManipulation (1)
                SegQueue::new(), // Rendering (2)
                SegQueue::new(), // HistoryTraversal (3)
                SegQueue::new(), // Networking (4)
                SegQueue::new(), // Timer (5)
                SegQueue::new(), // Internal (6)
            ],
        }
    }

    #[inline]
    fn push(&self, task: Task) {
        let idx = task.source.priority() as usize;
        if idx < NUM_TASK_SOURCES {
            self.queues[idx].push(task);
        } else {
            self.queues[NUM_TASK_SOURCES - 1].push(task);
        }
    }

    /// Seleciona a próxima tarefa prioritária respeitando a especificação WHATWG.
    #[inline]
    fn pop_highest_priority(&self) -> Option<Task> {
        for q in &self.queues {
            if let Some(task) = q.pop() {
                return Some(task);
            }
        }
        None
    }
}

/// Handle clonável para submissão de tarefas e microtasks a partir de qualquer thread.
#[derive(Clone)]
pub struct TaskQueue {
    /// Filas concorrentes dedicadas por `TaskSource`.
    task_queues: Arc<TaskSourceQueues>,
    /// Fila concorrente dinâmica ilimitada para microtasks (zero perda de promises).
    microtask_queue: Arc<SegQueue<TaskFn>>,
    /// Fila de tarefas de renderização (requestAnimationFrame).
    raf_queue: Arc<SegQueue<TaskFn>>,
    /// Fila de tarefas ociosas (requestIdleCallback).
    idle_queue: Arc<IdleTaskQueue>,
    /// Heap mínima de temporizadores agendados ($O(\log n)$).
    timers: Arc<Mutex<BinaryHeap<ScheduledTimer>>>,
    /// Flag para throttling de abas em segundo plano (min 1000ms para timers).
    background_throttling: Arc<AtomicBool>,
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
        self.task_queues.push(task);
        (id, cancel_handle)
    }

    /// Enfileira uma tarefa de interação do usuário (alta prioridade: cliques, digitação).
    pub fn queue_user_interaction<F>(&self, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::UserInteraction, f)
    }

    /// Enfileira uma tarefa de manipulação de DOM.
    pub fn queue_dom<F>(&self, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::DomManipulation, f)
    }

    /// Enfileira uma tarefa de rede (`fetch`, websockets, callbacks HTTP).
    pub fn queue_network<F>(&self, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::Networking, f)
    }

    /// Enfileira uma tarefa de navegação / histórico.
    pub fn queue_history<F>(&self, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::HistoryTraversal, f)
    }

    /// Enfileira uma tarefa de pipeline de renderização.
    pub fn queue_rendering<F>(&self, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::Rendering, f)
    }

    /// Enfileira uma tarefa de temporizador imediato.
    pub fn queue_timer<F>(&self, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue_task(TaskSource::Timer, f)
    }

    /// Agenda um temporizador para disparar após um atraso (`delay`) no Event Loop (`setTimeout`).
    /// Aplica regras de clamping da WHATWG HTML (nível de aninhamento >= 5 exige min 4ms).
    pub fn schedule_timer<F>(&self, delay: Duration, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce() + Send + 'static,
    {
        self.schedule_timer_with_nesting(delay, 1, f)
    }

    /// Agenda um temporizador com nível de aninhamento explícito para clamping conforme a spec.
    pub fn schedule_timer_with_nesting<F>(
        &self,
        delay: Duration,
        nesting_level: usize,
        f: F,
    ) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce() + Send + 'static,
    {
        let id = TaskId::new();
        let is_cancelled = Arc::new(AtomicBool::new(false));

        let mut effective_delay_ms = delay.as_millis() as u64;

        // Clamping WHATWG HTML 8.5.2: Timers com aninhamento >= 5 são limitados a no mínimo 4ms
        if nesting_level >= 5 && effective_delay_ms < 4 {
            effective_delay_ms = 4;
        }

        // Throttling de abas em segundo plano: timers limitados a no mínimo 1000ms
        if self.background_throttling.load(Ordering::Relaxed) && effective_delay_ms < 1000 {
            effective_delay_ms = 1000;
        }

        let target_ms = self.clock.now_ms() + effective_delay_ms;

        let timer = ScheduledTimer {
            id,
            target_ms,
            nesting_level,
            is_cancelled: Arc::clone(&is_cancelled),
            func: Box::new(f),
        };

        self.timers.lock().push(timer);
        (id, is_cancelled)
    }

    /// Ativa ou desativa o modo de throttling de segundo plano para esta fila de tarefas.
    pub fn set_background_throttling(&self, enabled: bool) {
        self.background_throttling.store(enabled, Ordering::Relaxed);
    }

    /// Retorna `true` se o throttling em segundo plano estiver ativo.
    pub fn is_background_throttling(&self) -> bool {
        self.background_throttling.load(Ordering::Relaxed)
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

    /// Enfileira uma tarefa ociosa (`requestIdleCallback`) com timeout opcional.
    pub fn post_idle_task<F>(&self, timeout: Option<Duration>, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce(IdleDeadline) + Send + 'static,
    {
        self.idle_queue.post_idle_task(timeout, &*self.clock, f)
    }

    /// Cria um novo `TaskScope` vinculado a esta fila para gerenciamento de ciclo de vida de abas/frames.
    pub fn create_scope(&self) -> (TaskScope, ScopedTaskQueue) {
        let scope = TaskScope::new();
        let scoped_queue = scope.wrap_queue(self.clone());
        (scope, scoped_queue)
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
    task_queues: Arc<TaskSourceQueues>,
    microtask_queue: Arc<SegQueue<TaskFn>>,
    raf_queue: Arc<SegQueue<TaskFn>>,
    idle_queue: Arc<IdleTaskQueue>,
    timers: Arc<Mutex<BinaryHeap<ScheduledTimer>>>,
    background_throttling: Arc<AtomicBool>,
    is_running: Arc<AtomicBool>,
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
        let task_queues = Arc::new(TaskSourceQueues::new());
        let microtask_queue = Arc::new(SegQueue::new());
        let raf_queue = Arc::new(SegQueue::new());
        let idle_queue = Arc::new(IdleTaskQueue::new());
        let timers = Arc::new(Mutex::new(BinaryHeap::new()));
        let background_throttling = Arc::new(AtomicBool::new(false));
        let is_running = Arc::new(AtomicBool::new(false));

        let handle = TaskQueue {
            task_queues: Arc::clone(&task_queues),
            microtask_queue: Arc::clone(&microtask_queue),
            raf_queue: Arc::clone(&raf_queue),
            idle_queue: Arc::clone(&idle_queue),
            timers: Arc::clone(&timers),
            background_throttling: Arc::clone(&background_throttling),
            clock: Arc::clone(&clock),
        };

        Self {
            task_queues,
            microtask_queue,
            raf_queue,
            idle_queue,
            timers,
            background_throttling,
            is_running,
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

    /// Ativa ou desativa o throttling de timers para abas em segundo plano.
    pub fn set_background_throttling(&self, enabled: bool) {
        self.background_throttling.store(enabled, Ordering::Relaxed);
    }

    /// Processa todos os temporizadores agendados cujo prazo já expirou utilizando a Min-Heap ($O(k \log n)$).
    pub fn process_expired_timers(&self) -> usize {
        let now = self.clock.now_ms();
        let mut expired = Vec::new();

        {
            let mut timers = self.timers.lock();
            while let Some(timer) = timers.peek() {
                if timer.target_ms <= now {
                    if let Some(t) = timers.pop() {
                        expired.push(t);
                    }
                } else {
                    break;
                }
            }
        }

        let count = expired.len();
        for timer in expired {
            if !timer
                .is_cancelled
                .load(std::sync::atomic::Ordering::Relaxed)
            {
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

        // 2. Seleciona e executa a Macrotask de maior prioridade disponível
        if let Some(task) = self.task_queues.pop_highest_priority() {
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

    /// Processa tarefas ociosas (`requestIdleCallback`) respeitando um orçamento máximo de tempo.
    pub fn process_idle_tasks(&self, max_budget: Duration) -> usize {
        let executed = self
            .idle_queue
            .process_idle_tasks(Arc::clone(&self.clock), max_budget);
        if executed > 0 {
            self.drain_microtasks();
        }
        executed
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

    /// Solicita o encerramento do Event Loop contínuo.
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// Inicia a execução contínua bloqueando a thread atual até `stop()` ser chamado.
    pub fn run_sync(&self) {
        debug!("Iniciando Event Loop WHATWG...");
        self.is_running.store(true, Ordering::SeqCst);

        while self.is_running.load(Ordering::SeqCst) {
            let processed = self.step();
            if !processed {
                std::thread::yield_now();
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
