//! # Event Loop (WHATWG Compliant)
//!
//! O coração de cada Aba de navegador. O Javascript e as renderizações não podem rodar 
//! simultaneamente sem regras estritas, senão a página colapsa em Data Races visuais.
//!
//! A especificação do WHATWG determina que:
//! 1. O motor deve pegar **uma e apenas uma** Macrotask (ex: um clique do mouse, ou resposta de rede).
//! 2. Executá-la.
//! 3. Em seguida, deve drenar **toda a fila de Microtasks** (Promises e Mutation Observers).
//! 4. (Opcional) Executar o pipeline de Renderização (recalcular estilos, desenhar).
//! 5. Voltar ao passo 1.
//!
//! Este módulo roda em cima de um `tokio::Runtime`, garantindo que toda a espera 
//! por I/O seja ultra-otimizada via kernel (epoll/kqueue) enquanto mantemos a ordem de execução do JS.

use crossbeam::queue::ArrayQueue;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, trace};

type TaskFn = Box<dyn FnOnce() + Send + 'static>;

/// Filas de submissão. Para injetar tarefas no EventLoop a partir de outras threads (ex: rede).
#[derive(Clone)]
pub struct TaskQueue {
    macrotask_tx: mpsc::UnboundedSender<TaskFn>,
    microtask_queue: Arc<ArrayQueue<TaskFn>>,
}

impl TaskQueue {
    /// Enfileira uma macrotask (clique do mouse, `setTimeout`, evento de I/O).
    pub fn queue_macrotask<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let _ = self.macrotask_tx.send(Box::new(f));
    }

    /// Enfileira uma microtask (Promise, Mutation Observer).
    /// Executam com prioridade absoluta ao final da macrotask atual.
    pub fn queue_microtask<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // Se a fila lotar (ex: script bizarro injetando 1 milhão de promises síncronas),
        // ele ignora ou dá falha silenciosa. Em produção, usaríamos uma fila ilimitada ou 
        // mataríamos o script. Para o MVP, 10_000 é suficiente.
        if self.microtask_queue.push(Box::new(f)).is_err() {
            tracing::error!("OOM: Fila de Microtasks excedeu a capacidade!");
        }
    }
}

/// O Motor que processa as filas de forma contínua e ordenada.
pub struct EventLoop {
    macrotask_rx: mpsc::UnboundedReceiver<TaskFn>,
    microtask_queue: Arc<ArrayQueue<TaskFn>>,
    queue_handle: TaskQueue,
}

impl EventLoop {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let micro_queue = Arc::new(ArrayQueue::new(10_000));
        
        let handle = TaskQueue {
            macrotask_tx: tx,
            microtask_queue: Arc::clone(&micro_queue),
        };

        Self {
            macrotask_rx: rx,
            microtask_queue: micro_queue,
            queue_handle: handle,
        }
    }

    /// Retorna um handle que pode ser clonado e enviado para outras threads
    /// para alimentar este Event Loop com trabalho.
    pub fn handle(&self) -> TaskQueue {
        self.queue_handle.clone()
    }

    /// Inicia o processamento síncrono. Bloqueia a thread atual.
    /// Em um cenário real, isso rodaria dentro de uma thread dedicada de renderização.
    pub fn run_sync(&mut self) {
        debug!("Iniciando Event Loop WHATWG...");

        // 1. Bloqueia até chegar a próxima Macrotask.
        while let Some(macro_task) = self.macrotask_rx.blocking_recv() {
            trace!("Executando Macrotask...");
            macro_task();

            // 2. Após UMA macrotask, drena TODA a fila de Microtasks.
            self.drain_microtasks();
            
            // 3. Aqui entraria o Pipeline de Renderização (rAF)
            // if needs_render { render_frame(); }
        }
        
        debug!("Event Loop encerrado.");
    }

    /// Drena a fila de microtasks até ela ficar vazia.
    /// Nota: Microtasks podem agendar mais microtasks, este loop roda até o esgotamento total.
    fn drain_microtasks(&self) {
        let mut executed = 0;
        while let Some(micro_task) = self.microtask_queue.pop() {
            trace!("Executando Microtask...");
            micro_task();
            executed += 1;
            
            // Circuit Breaker para evitar travamento infinito por Promises que chamam a si mesmas
            if executed > 100_000 {
                tracing::error!("Circuit Breaker: Esgotamento de limite de microtasks (Infinite Promise Loop?)");
                break;
            }
        }
    }
}

impl Default for EventLoop {
    fn default() -> Self {
        Self::new()
    }
}


