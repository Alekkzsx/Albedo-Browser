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
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, trace, warn};

thread_local! {
    /// Rastreia o nível de aninhamento do timer em execução na thread atual (WHATWG §8.5.2).
    static CURRENT_TIMER_NESTING: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Retorna o nível de aninhamento de timer atual na thread corrente.
#[inline]
pub fn current_timer_nesting() -> usize {
    CURRENT_TIMER_NESTING.with(|c| c.get())
}

/// Número total de fontes de tarefas padronizadas no WHATWG HTML.
pub const NUM_TASK_SOURCES: usize = TaskSource::COUNT;

/// Limite padrão de starvation (número de passos que uma fila com tarefas pendentes pode ser preterida
/// antes de ser forçada a executar, prevenindo starvation conforme WHATWG §8.1.6).
pub const DEFAULT_STARVATION_LIMIT: u32 = 5;

/// Representa um temporizador agendado (`setTimeout` / `setInterval`) ordenado por prazo.
struct ScheduledTimer {
    id: TaskId,
    target_ms: u64,
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

/// Conjunto de filas concorrentes dedicadas por fonte de tarefas WHATWG com escalonador justo (Fair Queuing)
/// e prevenção de starvation por envelhecimento (Aging).
pub struct TaskSourceQueues {
    queues: [SegQueue<Task>; NUM_TASK_SOURCES],
    starvation_counters: [AtomicU32; NUM_TASK_SOURCES],
    starvation_limit: u32,
}

impl Default for TaskSourceQueues {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskSourceQueues {
    pub fn new() -> Self {
        Self::with_starvation_limit(DEFAULT_STARVATION_LIMIT)
    }


    pub fn with_starvation_limit(starvation_limit: u32) -> Self {
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
            starvation_counters: [
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
            ],
            starvation_limit,
        }
    }

    #[inline]
    pub fn push(&self, task: Task) {
        let idx = task.source.priority() as usize;
        if idx < NUM_TASK_SOURCES {
            self.queues[idx].push(task);
        } else {
            self.queues[NUM_TASK_SOURCES - 1].push(task);
        }
    }

    /// Retorna o valor atual do contador de starvation para uma dada fonte de tarefas.
    #[inline]
    pub fn starvation_counter(&self, source: TaskSource) -> u32 {
        let idx = source.priority() as usize;
        if idx < NUM_TASK_SOURCES {
            self.starvation_counters[idx].load(Ordering::Relaxed)
        } else {
            0
        }
    }

    /// Seleciona a próxima tarefa prioritária respeitando a especificação WHATWG com prevenção de starvation.
    pub fn pop_highest_priority(&self) -> Option<Task> {
        // 1. Prevenção de Starvation: verifica se alguma fila não-vazia atingiu o limite de starvation
        let mut starved_idx = None;
        let mut max_starvation = self.starvation_limit;

        for (idx, counter) in self.starvation_counters.iter().enumerate() {
            let val = counter.load(Ordering::Acquire);
            if val >= max_starvation && !self.queues[idx].is_empty() {
                max_starvation = val;
                starved_idx = Some(idx);
            }
        }

        if let Some(idx) = starved_idx {
            if let Some(task) = self.queues[idx].pop() {
                self.starvation_counters[idx].store(0, Ordering::Release);
                for (i, q) in self.queues.iter().enumerate() {
                    if i != idx {
                        if !q.is_empty() {
                            self.starvation_counters[i].fetch_add(1, Ordering::Relaxed);
                        } else {
                            self.starvation_counters[i].store(0, Ordering::Relaxed);
                        }
                    }
                }
                return Some(task);
            } else {
                self.starvation_counters[idx].store(0, Ordering::Release);
            }
        }

        // 2. Se nenhuma fila estiver faminta, despacha por ordem padrão de prioridade (0..6)
        for (idx, q) in self.queues.iter().enumerate() {
            if let Some(task) = q.pop() {
                self.starvation_counters[idx].store(0, Ordering::Release);
                for (i, other_q) in self.queues.iter().enumerate() {
                    if i != idx {
                        if !other_q.is_empty() {
                            self.starvation_counters[i].fetch_add(1, Ordering::Relaxed);
                        } else {
                            self.starvation_counters[i].store(0, Ordering::Relaxed);
                        }
                    }
                }
                return Some(task);
            } else {
                self.starvation_counters[idx].store(0, Ordering::Relaxed);
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
        let current_nesting = CURRENT_TIMER_NESTING.with(|c| c.get());
        let nesting_level = current_nesting + 1;
        self.schedule_timer_with_nesting(delay, nesting_level, f)
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
    pub task_queues: Arc<TaskSourceQueues>,
    microtask_queue: Arc<SegQueue<TaskFn>>,
    raf_queue: Arc<SegQueue<TaskFn>>,
    idle_queue: Arc<IdleTaskQueue>,
    timers: Arc<Mutex<BinaryHeap<ScheduledTimer>>>,
    background_throttling: Arc<AtomicBool>,
    is_running: Arc<AtomicBool>,
    performing_microtask_checkpoint: Arc<AtomicBool>,
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
        Self::with_clock_and_starvation_limit(clock, DEFAULT_STARVATION_LIMIT)
    }

    /// Inicializa um Event Loop com um relógio customizado e um limite específico de starvation.
    pub fn with_clock_and_starvation_limit(clock: Arc<dyn Clock>, starvation_limit: u32) -> Self {
        let task_queues = Arc::new(TaskSourceQueues::with_starvation_limit(starvation_limit));
        let microtask_queue = Arc::new(SegQueue::new());
        let raf_queue = Arc::new(SegQueue::new());
        let idle_queue = Arc::new(IdleTaskQueue::new());
        let timers = Arc::new(Mutex::new(BinaryHeap::new()));
        let background_throttling = Arc::new(AtomicBool::new(false));
        let is_running = Arc::new(AtomicBool::new(false));
        let performing_microtask_checkpoint = Arc::new(AtomicBool::new(false));

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
            performing_microtask_checkpoint,
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

    /// Retorna `true` se um Microtask Checkpoint estiver sendo executado atualmente neste Event Loop (WHATWG §8.1.6.3).
    #[inline]
    pub fn is_performing_microtask_checkpoint(&self) -> bool {
        self.performing_microtask_checkpoint.load(Ordering::Acquire)
    }

    /// Ativa ou desativa o throttling de timers para abas em segundo plano.
    pub fn set_background_throttling(&self, enabled: bool) {
        self.background_throttling.store(enabled, Ordering::Relaxed);
    }

    /// Processa todos os temporizadores agendados cujo prazo já expirou utilizando a Min-Heap ($O(k \log n)$),
    /// enfileirando cada timer como uma macrotask individual no `TaskSource::Timer` (WHATWG §8.1.6).
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
            let nesting = timer.nesting_level;
            let func = timer.func;
            let is_cancelled = timer.is_cancelled;
            let id = timer.id;

            let task = Task {
                id,
                source: TaskSource::Timer,
                is_cancelled,
                func: Box::new(move || {
                    let prev_nesting = CURRENT_TIMER_NESTING.with(|c| c.replace(nesting));
                    struct NestingScope(usize);
                    impl Drop for NestingScope {
                        fn drop(&mut self) {
                            CURRENT_TIMER_NESTING.with(|c| c.set(self.0));
                        }
                    }
                    let _scope = NestingScope(prev_nesting);
                    func();
                }),
            };

            self.task_queues.push(task);
        }

        count
    }

    /// Executa um passo único do Event Loop (WHATWG §8.1.6).
    ///
    /// 1. Promove temporizadores expirados para a fila de macrotasks `TaskSource::Timer`.
    /// 2. Seleciona e executa exatamente UMA macrotask disponível (com prevenção de starvation).
    /// 3. Executa o Microtask Checkpoint (`drain_microtasks()`) imediatamente após a macrotask.
    ///
    /// Retorna `true` se alguma tarefa, timer ou microtask foi processada.
    pub fn step(&self) -> bool {
        let mut executed_something = false;

        // 1. Processa/enfileira temporizadores cujo prazo expirou
        let expired_count = self.process_expired_timers();
        if expired_count > 0 {
            executed_something = true;
        }

        // 2. Seleciona e executa exatamente UMA Macrotask de maior prioridade disponível
        if let Some(task) = self.task_queues.pop_highest_priority() {
            trace!("Executando Macrotask [{:?}] ID: {}", task.source, task.id);
            task.execute();
            executed_something = true;
        }

        // 3. Microtask Checkpoint: drena todas as microtasks pendentes após a macrotask
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

    /// Drena exaustivamente a fila de microtasks (Promises) com Reentrancy Guard (WHATWG §8.1.6.3).
    pub fn drain_microtasks(&self) -> usize {
        // WHATWG HTML §8.1.6.3 Step 1: Se já estiver executando um checkpoint, retorna imediatamente.
        if self
            .performing_microtask_checkpoint
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return 0;
        }

        // RAII Guard para garantir reset da flag mesmo sob panic ou retorno antecipado
        struct MicrotaskGuard<'a>(&'a AtomicBool);
        impl<'a> Drop for MicrotaskGuard<'a> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _guard = MicrotaskGuard(&self.performing_microtask_checkpoint);

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
