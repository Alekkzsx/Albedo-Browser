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

pub mod budget;
pub mod coalescer;
pub mod idle;
pub mod scope;
pub mod source;
pub mod task;
pub mod utils;

pub use budget::{
    AntiStarvationSelector, BackgroundTabThrottler, CPUTimeBudgetPool, TaskPriority,
};
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
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};
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

/// Conjunto de filas concorrentes dedicadas por fonte de tarefas WHATWG com escalonador justo (Fair Queuing),
/// máscara de bits atômica para despacho $O(1)$ amortizado e prevenção de starvation por envelhecimento (Aging).
pub struct TaskSourceQueues {
    queues: [SegQueue<Task>; NUM_TASK_SOURCES],
    starvation_counters: [AtomicU32; NUM_TASK_SOURCES],
    active_mask: AtomicU8,
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
            active_mask: AtomicU8::new(0),
            starvation_limit,
        }
    }

    #[inline]
    pub fn push(&self, task: Task) {
        let idx = task.source.priority() as usize;
        let target = if idx < NUM_TASK_SOURCES { idx } else { NUM_TASK_SOURCES - 1 };
        self.queues[target].push(task);
        self.active_mask.fetch_or(1 << target, Ordering::Release);
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
        let mut mask = self.active_mask.load(Ordering::Acquire);
        if mask == 0 {
            return None;
        }

        // 1. Prevenção de Starvation: verifica se alguma fila ativa atingiu o limite de starvation
        let mut starved_idx = None;
        let mut max_starvation = self.starvation_limit;

        for (idx, counter) in self.starvation_counters.iter().enumerate() {
            if (mask & (1 << idx)) != 0 {
                let val = counter.load(Ordering::Relaxed);
                if val >= max_starvation && !self.queues[idx].is_empty() {
                    max_starvation = val;
                    starved_idx = Some(idx);
                }
            }
        }

        if let Some(idx) = starved_idx {
            if let Some(task) = self.queues[idx].pop() {
                self.starvation_counters[idx].store(0, Ordering::Release);
                if self.queues[idx].is_empty() {
                    self.active_mask.fetch_and(!(1 << idx), Ordering::Release);
                }
                for (i, q) in self.queues.iter().enumerate() {
                    if i != idx {
                        if !q.is_empty() {
                            self.starvation_counters[i].fetch_add(1, Ordering::Relaxed);
                        } else {
                            self.starvation_counters[i].store(0, Ordering::Relaxed);
                            self.active_mask.fetch_and(!(1 << i), Ordering::Relaxed);
                        }
                    }
                }
                return Some(task);
            } else {
                self.starvation_counters[idx].store(0, Ordering::Release);
                self.active_mask.fetch_and(!(1 << idx), Ordering::Release);
            }
        }

        // 2. Se nenhuma fila estiver faminta, despacha pela fila de maior prioridade disponível via bitmask
        while mask != 0 {
            let idx = mask.trailing_zeros() as usize;
            if idx >= NUM_TASK_SOURCES {
                break;
            }

            if let Some(task) = self.queues[idx].pop() {
                self.starvation_counters[idx].store(0, Ordering::Release);
                if self.queues[idx].is_empty() {
                    self.active_mask.fetch_and(!(1 << idx), Ordering::Release);
                }
                for (i, other_q) in self.queues.iter().enumerate() {
                    if i != idx {
                        if !other_q.is_empty() {
                            self.starvation_counters[i].fetch_add(1, Ordering::Relaxed);
                        } else {
                            self.starvation_counters[i].store(0, Ordering::Relaxed);
                            self.active_mask.fetch_and(!(1 << i), Ordering::Relaxed);
                        }
                    }
                }
                return Some(task);
            } else {
                self.starvation_counters[idx].store(0, Ordering::Relaxed);
                self.active_mask.fetch_and(!(1 << idx), Ordering::Release);
                mask &= !(1 << idx);
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
    /// Prazo do temporizador mais próximo em milissegundos (fast-path lock-free).
    earliest_deadline_ms: Arc<AtomicU64>,
    /// Quantidade total de temporizadores pendentes (fast-path lock-free).
    timer_count: Arc<AtomicU32>,
    /// Flag para throttling de abas em segundo plano (min 1000ms para timers).
    background_throttling: Arc<AtomicBool>,
    /// Relógio de referência para resolução de prazos.
    clock: Arc<dyn Clock>,
    /// Flag atômica indicando se o Event Loop está atualmente adormecido.
    is_sleeping: Arc<AtomicBool>,
    /// Notificador para acordar a thread do Event Loop imediatamente sem busy waiting.
    waker: Arc<(Mutex<()>, parking_lot::Condvar)>,
}

impl TaskQueue {
    /// Acorda o Event Loop adormecido imediatamente caso esteja em espera condvar.
    #[inline(always)]
    pub fn wake(&self) {
        if self.is_sleeping.load(Ordering::Acquire) {
            self.waker.1.notify_one();
        }
    }

    /// Enfileira uma macrotask associada a uma fonte específica da spec WHATWG.
    /// Retorna o `TaskId` e o controle atômico de cancelamento (`is_cancelled`).
    pub fn queue_task<F>(&self, source: TaskSource, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce() + Send + 'static,
    {
        let (task, cancel_handle) = Task::new(source, f);
        let id = task.id;
        self.task_queues.push(task);
        self.wake();
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
        self.timer_count.fetch_add(1, Ordering::Release);
        self.earliest_deadline_ms.fetch_min(target_ms, Ordering::Release);
        self.wake();
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
    pub fn queue_microtask<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.microtask_queue.push(Box::new(f));
        self.wake();
    }

    /// Enfileira uma função para ser executada no próximo quadro de animação (`requestAnimationFrame`).
    pub fn request_animation_frame<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.raf_queue.push(Box::new(f));
        self.wake();
    }

    /// Enfileira uma tarefa ociosa (`requestIdleCallback`) com timeout opcional.
    pub fn post_idle_task<F>(&self, timeout: Option<Duration>, f: F) -> (TaskId, Arc<AtomicBool>)
    where
        F: FnOnce(IdleDeadline) + Send + 'static,
    {
        let res = self.idle_queue.post_idle_task(timeout, &*self.clock, f);
        self.wake();
        res
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
    earliest_deadline_ms: Arc<AtomicU64>,
    timer_count: Arc<AtomicU32>,
    background_throttling: Arc<AtomicBool>,
    is_running: Arc<AtomicBool>,
    performing_microtask_checkpoint: Arc<AtomicBool>,
    clock: Arc<dyn Clock>,
    is_sleeping: Arc<AtomicBool>,
    waker: Arc<(Mutex<()>, parking_lot::Condvar)>,
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
        let earliest_deadline_ms = Arc::new(AtomicU64::new(u64::MAX));
        let timer_count = Arc::new(AtomicU32::new(0));
        let background_throttling = Arc::new(AtomicBool::new(false));
        let is_running = Arc::new(AtomicBool::new(false));
        let performing_microtask_checkpoint = Arc::new(AtomicBool::new(false));
        let is_sleeping = Arc::new(AtomicBool::new(false));
        let waker = Arc::new((Mutex::new(()), parking_lot::Condvar::new()));

        let handle = TaskQueue {
            task_queues: Arc::clone(&task_queues),
            microtask_queue: Arc::clone(&microtask_queue),
            raf_queue: Arc::clone(&raf_queue),
            idle_queue: Arc::clone(&idle_queue),
            timers: Arc::clone(&timers),
            earliest_deadline_ms: Arc::clone(&earliest_deadline_ms),
            timer_count: Arc::clone(&timer_count),
            background_throttling: Arc::clone(&background_throttling),
            clock: Arc::clone(&clock),
            is_sleeping: Arc::clone(&is_sleeping),
            waker: Arc::clone(&waker),
        };

        Self {
            task_queues,
            microtask_queue,
            raf_queue,
            idle_queue,
            timers,
            earliest_deadline_ms,
            timer_count,
            background_throttling,
            is_running,
            performing_microtask_checkpoint,
            clock,
            is_sleeping,
            waker,
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

    /// Acorda o Event Loop adormecido imediatamente.
    #[inline(always)]
    pub fn wake(&self) {
        if self.is_sleeping.load(Ordering::Acquire) {
            self.waker.1.notify_one();
        }
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
    /// com fast-path lock-free eliminando travamento de Mutex em quadros sem timers pendentes.
    pub fn process_expired_timers(&self) -> usize {
        if self.timer_count.load(Ordering::Relaxed) == 0 {
            return 0;
        }

        let now = self.clock.now_ms();
        if now < self.earliest_deadline_ms.load(Ordering::Relaxed) {
            return 0;
        }

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

            let remaining_count = timers.len() as u32;
            self.timer_count.store(remaining_count, Ordering::Release);
            if let Some(earliest) = timers.peek() {
                self.earliest_deadline_ms.store(earliest.target_ms, Ordering::Release);
            } else {
                self.earliest_deadline_ms.store(u64::MAX, Ordering::Release);
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
        if self
            .performing_microtask_checkpoint
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return 0;
        }

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
        self.waker.1.notify_all();
    }

    /// Inicia a execução contínua bloqueando a thread atual até `stop()` ser chamado.
    /// Utiliza suspensão eficiente (sem busy waiting / zero 100% CPU spinning).
    pub fn run_sync(&self) {
        debug!("Iniciando Event Loop WHATWG...");
        self.is_running.store(true, Ordering::SeqCst);

        while self.is_running.load(Ordering::SeqCst) {
            let processed = self.step();
            if !processed {
                let next_timeout = {
                    let earliest_ms = self.earliest_deadline_ms.load(Ordering::Relaxed);
                    if earliest_ms != u64::MAX {
                        let now = self.clock.now_ms();
                        let remaining = earliest_ms.saturating_sub(now);
                        Duration::from_millis(remaining.clamp(1, 50))
                    } else {
                        Duration::from_millis(50)
                    }
                };

                let mut guard = self.waker.0.lock();
                self.is_sleeping.store(true, Ordering::Release);
                self.waker.1.wait_for(&mut guard, next_timeout);
                self.is_sleeping.store(false, Ordering::Release);
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
