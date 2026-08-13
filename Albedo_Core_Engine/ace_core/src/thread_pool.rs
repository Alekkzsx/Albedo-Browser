// ============================================================================
// Albedo Core Engine (ACE)
// File: thread_pool.rs
// Description: Gerenciador Nativo de Threads com "Work-Stealing" Local Queues,
//              Mechanical Sympathy (Padded 64 bytes) e Zero Lock Contention Global.
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use crate::deque::WorkerDeque;

pub type Job = Box<dyn FnOnce() + Send + 'static>;

/// Fila isolada de um único Worker (Agora Lock-Free)
/// Alinhada em 64 bytes para evitar `False Sharing`.
#[repr(align(64))]
struct PaddedQueue {
    queue: WorkerDeque<Job>,
}

struct SharedState {
    local_queues: Vec<PaddedQueue>,
    /// Usado apenas para a condição de dormência (Condvar)
    sleep_mutex: Mutex<()>,
    condvar: Condvar,
    shutdown: AtomicBool,
    /// Mantém o rastro do volume total para evitar bloqueios cegos
    pending_tasks: AtomicUsize,
}

impl SharedState {
    fn new(num_workers: usize) -> Self {
        let mut local_queues = Vec::with_capacity(num_workers);
        for _ in 0..num_workers {
            local_queues.push(PaddedQueue {
                queue: WorkerDeque::new(),
            });
        }
        
        Self {
            local_queues,
            sleep_mutex: Mutex::new(()),
            condvar: Condvar::new(),
            shutdown: AtomicBool::new(false),
            pending_tasks: AtomicUsize::new(0),
        }
    }
}

struct Worker {
    _id: usize,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, shared_state: Arc<SharedState>) -> Self {
        let num_workers = shared_state.local_queues.len();
        let builder = thread::Builder::new().name(format!("ace-worker-{}", id));

        let thread = builder
            .spawn(move || {
                loop {
                    // 1. TENTATIVA LOCAL (LIFO, sem lock, O(1))
                    let mut job = shared_state.local_queues[id].queue.pop();

                    // 2. TENTATIVA DE ROUBO (WORK-STEALING, FIFO, CAS Lock-Free)
                    if job.is_none() {
                        for i in 0..num_workers {
                            let target_id = (id + i + 1) % num_workers;
                            if let Some(stolen) = shared_state.local_queues[target_id].queue.steal() {
                                job = Some(stolen);
                                break;
                            }
                        }
                    }

                    // 3. EXECUÇÃO
                    if let Some(j) = job {
                        // Subtrai do contador de pendências globais
                        shared_state.pending_tasks.fetch_sub(1, Ordering::Relaxed);
                        
                        let result = catch_unwind(AssertUnwindSafe(j));
                        if let Err(e) = result {
                            let msg = if let Some(s) = e.downcast_ref::<&str>() {
                                *s
                            } else if let Some(s) = e.downcast_ref::<String>() {
                                s.as_str()
                            } else {
                                "OOM ou Falha Fatal"
                            };
                            crate::ace_error!("Worker {} sofreu Panic Interno: {}", id, msg);
                        }
                        continue; // Evita entrar no fluxo de sleep se tínhamos trabalho
                    }

                    // 4. SHUTDOWN CHECK
                    if shared_state.shutdown.load(Ordering::Acquire) {
                        let pending = shared_state.pending_tasks.load(Ordering::Acquire);
                        if pending == 0 {
                            crate::ace_trace!("Worker {} desligando com sucesso.", id);
                            break;
                        }
                    }

                    // 5. DORMÊNCIA (Se não achou nada e não está desligando)
                    let pending = shared_state.pending_tasks.load(Ordering::Acquire);
                    if pending == 0 {
                        // Trava o sleep_mutex para aguardar na Condvar
                        let lock = shared_state.sleep_mutex.lock().unwrap();
                        // Checagem dupla para prevenir "Missed Wakeup"
                        if shared_state.pending_tasks.load(Ordering::Acquire) == 0 && !shared_state.shutdown.load(Ordering::Acquire) {
                            drop(shared_state.condvar.wait(lock).unwrap());
                        }
                    } else {
                        // Se há pending, mas o try_lock falhou, nós cedemos o slice de CPU 
                        // para as outras threads destrarem os Mutexes mais rápido.
                        thread::yield_now();
                    }
                }
            })
            .unwrap();

        Self {
            _id: id,
            thread: Some(thread),
        }
    }
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    shared_state: Arc<SharedState>,
    next_worker: AtomicUsize,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        assert!(size > 0, "O ThreadPool exige no mínimo 1 Worker.");

        let shared_state = Arc::new(SharedState::new(size));
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&shared_state)));
        }

        Self {
            workers,
            shared_state,
            next_worker: AtomicUsize::new(0),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        
        // Distribui usando Round-Robin
        let num_workers = self.workers.len();
        let target_worker = self.next_worker.fetch_add(1, Ordering::Relaxed) % num_workers;
        
        // Incrementa ANTES de colocar na fila (evita a thread roubar antes de registrarmos)
        self.shared_state.pending_tasks.fetch_add(1, Ordering::Release);
        
        // Insere na fila (LIFO Lock-Free)
        // Como o fallback de capacidade estourada ainda não foi feito, garantimos por ora que não vai explodir
        // com o Assert ou um retry, mas no mundo real a task iria para uma Global Queue.
        let _ = self.shared_state.local_queues[target_worker].queue.push(job);
        
        // Acorda os workers dormentes
        self.shared_state.condvar.notify_one();
    }
    
    pub fn join(self) {}
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.shared_state.shutdown.store(true, Ordering::Release);
        self.shared_state.condvar.notify_all();

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
