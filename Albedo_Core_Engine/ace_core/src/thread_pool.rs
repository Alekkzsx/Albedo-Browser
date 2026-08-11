// ============================================================================
// Albedo Core Engine (ACE)
// File: thread_pool.rs
// Description: Gerenciador Nativo de Threads para execução assíncrona de
//              Macrotasks e Microtasks sem dependências externas (tokio/rayon).
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::collections::VecDeque;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

/// Representa uma tarefa enfileirada no pool.
/// O tipo `Box<dyn FnOnce()>` permite que os workers executem qualquer closure de forma genérica.
pub type Job = Box<dyn FnOnce() + Send + 'static>;

/// O estado imutável de memória que todas as threads compartilham,
/// sincronizado primitivamente via `Mutex` e acordado via `Condvar`.
struct ThreadPoolState {
    jobs: VecDeque<Job>,
    shutdown: bool,
}

struct SharedState {
    state: Mutex<ThreadPoolState>,
    condvar: Condvar,
}

impl SharedState {
    fn new() -> Self {
        Self {
            state: Mutex::new(ThreadPoolState {
                jobs: VecDeque::new(),
                shutdown: false,
            }),
            condvar: Condvar::new(),
        }
    }
}

/// Um Worker retém o controle sobre uma Thread real alocada no SO.
struct Worker {
    id: usize,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    /// Inicia uma nova thread do SO que ficará girando ad aeternum, consumindo a fila.
    fn new(id: usize, shared_state: Arc<SharedState>) -> Self {
        let builder = thread::Builder::new().name(format!("ace-worker-{}", id));

        let thread = builder
            .spawn(move || {
                loop {
                    let job = {
                        // Trava o acesso à fila
                        let mut state = shared_state.state.lock().unwrap();

                        // Enquanto a fila estiver vazia e não for para desligar, a thread dorme (0% CPU).
                        while state.jobs.is_empty() && !state.shutdown {
                            state = shared_state.condvar.wait(state).unwrap();
                        }

                        // Se recebemos a ordem de shutdown e esvaziamos a fila, o ciclo de vida desta thread chegou ao fim.
                        if state.shutdown && state.jobs.is_empty() {
                            crate::ace_trace!("Worker {} desligando com sucesso.", id);
                            break;
                        }

                        // Removemos a tarefa do topo da fila (Garantido de não falhar, pois o while bloqueou a execução)
                        state.jobs.pop_front().unwrap()
                    }; // <--- Mutex é automaticamente destravado aqui antes da tarefa rodar!

                    // --------------------------------------------------------
                    // Proteção de Pânico (Resiliência Crítica)
                    // --------------------------------------------------------
                    // Um erro fatal no JS JIT ou Layout não pode derrubar a Thread.
                    // Nós capturamos a explosão ("unwind") e simplesmente reportamos o log,
                    // mantendo a Thread perfeitamente saudável para pegar o próximo Job.
                    let result = catch_unwind(AssertUnwindSafe(job));
                    if let Err(e) = result {
                        let msg = if let Some(s) = e.downcast_ref::<&str>() {
                            *s
                        } else if let Some(s) = e.downcast_ref::<String>() {
                            s.as_str()
                        } else {
                            "Panic desconhecido ou falha irrecuperável de OOM."
                        };
                        crate::ace_error!("Worker {} sofreu Panic Interno: {}", id, msg);
                    }
                }
            })
            .unwrap();

        Self {
            id,
            thread: Some(thread),
        }
    }
}

/// A Interface Principal do Escalador de Tarefas do Albedo.
pub struct ThreadPool {
    workers: Vec<Worker>,
    shared_state: Arc<SharedState>,
}

impl ThreadPool {
    /// Inicializa o ThreadPool. Se `size` for 0, o sistema lerá a contagem
    /// nativa de processadores lógicos do Hardware.
    pub fn new(mut size: usize) -> Self {
        if size == 0 {
            size = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4); // Fallback conservador para 4 núcleos lógicos se a syscall falhar
        }

        crate::ace_debug!("Booting ACE ThreadPool com {} workers.", size);

        let shared_state = Arc::new(SharedState::new());
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&shared_state)));
        }

        Self {
            workers,
            shared_state,
        }
    }

    /// Envia uma *closure* pesada para a fila de execução concorrente e acorda um Worker nativo.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        {
            let mut state = self.shared_state.state.lock().unwrap();
            
            // Segurança: Se o pool já entrou em desligamento, bloqueamos a submissão.
            if state.shutdown {
                crate::ace_warn!("Aviso: Tentativa de `execute` após emissão de Shutdown.");
                return;
            }
            
            state.jobs.push_back(job);
        }
        
        // Acorda exatamente 1 Worker adormecido (se houver) via Sinal Nativo (Condvar).
        self.shared_state.condvar.notify_one();
    }
    
    /// Drena a fila atual graciosamente. Trava a thread de chamada até que
    /// o último Worker finalize seu trabalho pendente e saia limpo.
    pub fn join(self) {
        // Ao tomar `self` por valor, a estrutura é consumida.
        // O escopo encerra, e a trait `Drop` cuidará automaticamente da sincronização.
    }
}

/// Implementa a destruição limpa do pool, evitando vazamentos e
/// garantindo que todas as tarefas enfileiradas executem antes da morte térmica.
impl Drop for ThreadPool {
    fn drop(&mut self) {
        crate::ace_debug!("Iniciando Shutdown Graceful do ThreadPool...");

        {
            let mut state = self.shared_state.state.lock().unwrap();
            state.shutdown = true;
        }

        // Broad-cast global: "Acordem todos e leiam a bandeira de desligamento!"
        self.shared_state.condvar.notify_all();

        for worker in &mut self.workers {
            crate::ace_trace!("Aguardando encerramento final do Worker {}...", worker.id);
            if let Some(thread) = worker.thread.take() {
                let _ = thread.join();
            }
        }
        
        crate::ace_debug!("ThreadPool encerrado com sucesso.");
    }
}


