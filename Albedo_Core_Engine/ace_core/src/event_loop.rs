// ============================================================================
// Albedo Core Engine (ACE)
// File: event_loop.rs
// Description: O Event Loop central (inspirado na especificação WHATWG).
//              Gerencia a ordem global do tempo do navegador (Microtasks vs Macrotasks).
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::collections::VecDeque;
use std::sync::Arc;

use crate::io::IoMultiplexer;
use crate::thread_pool::{Job, ThreadPool};
use crate::time::MonotonicClock;

/// Representa a origem de uma Tarefa pesada (Macrotask).
/// Segue a taxonomia de filas do WHATWG.
pub enum Macrotask {
    Network(Job),
    UserInteraction(Job),
    Timer(Job),
    Rendering(Job),
}

/// O Ditador do Tempo. Estrutura primária que coordena a Thread Principal do navegador.
pub struct EventLoop<I: IoMultiplexer> {
    macrotasks: VecDeque<Macrotask>,
    microtasks: VecDeque<Job>,
    multiplexer: I,
    pub thread_pool: Arc<ThreadPool>,
    last_render_time: u64,
    should_exit: bool,
}

impl<I: IoMultiplexer> EventLoop<I> {
    pub fn new(multiplexer: I, thread_pool: Arc<ThreadPool>) -> Self {
        Self {
            macrotasks: VecDeque::new(),
            microtasks: VecDeque::new(),
            multiplexer,
            thread_pool,
            last_render_time: 0,
            should_exit: false,
        }
    }

    /// Sinaliza o EventLoop para que seja encerrado graciosamente após o ciclo atual.
    pub fn exit(&mut self) {
        self.should_exit = true;
    }

    /// Enfileira uma Macrotask. (Ex: Um pacote TCP terminou de baixar)
    pub fn queue_macrotask(&mut self, task: Macrotask) {
        self.macrotasks.push_back(task);
    }

    /// Enfileira uma Microtask. (Ex: Uma Promise de JavaScript foi resolvida)
    /// Microtasks furam a fila das Macrotasks, sendo esvaziadas completamente na mesma iteração.
    pub fn queue_microtask(&mut self, task: Job) {
        self.microtasks.push_back(task);
    }

    /// Executa um ciclo atômico do Event Loop seguindo rigidamente o HTML Standard.
    pub fn run_once(&mut self) -> crate::AceResult<()> {
        // 1. MACROTASK: Executa exatamente UMA (se houver) para garantir justiça no uso de CPU.
        if let Some(macro_task) = self.macrotasks.pop_front() {
            match macro_task {
                Macrotask::Network(job)
                | Macrotask::UserInteraction(job)
                | Macrotask::Timer(job)
                | Macrotask::Rendering(job) => {
                    // Executamos IN-PLACE (na Main Thread) para garantir a ordem correta das Microtasks!
                    // Tarefas CPU-bound intensas devem ser enviadas manualmente ao `thread_pool` dentro do `job`.
                    job();
                }
            }
        }

        // 2. MICROTASKS: O Dreno Absoluto (Run to completion).
        // Todas as Microtasks pendentes (ex: ".then()" de Promises) rodam AGORA, in-place.
        while let Some(micro_task) = self.microtasks.pop_front() {
            // Em um motor JS real (Fase 10), rodaremos in-place.
            // Para não travar a main-thread neste momento inicial, enviamos ao ThreadPool
            // mas aguardamos (blocking) se necessário, ou assumimos arquitetura assíncrona.
            // Vamos executar in-place para respeitar a especificação do JS.
            micro_task();
        }

        // 3. RENDERIZAÇÃO: Controle de Frame Rate (60 FPS = 16.6ms)
        let current_time = MonotonicClock::now_ms();
        let elapsed = current_time.saturating_sub(self.last_render_time);

        if elapsed >= 16 {
            self.last_render_time = current_time;
            // No futuro, isso dispararia o Recalculate Style, Layout e Paint (Fases 6, 7 e 8).
            crate::ace_trace!("EventLoop: VSync -> Disparando Frame de Renderização (60 FPS).");
        }

        // 4. I/O MULTIPLEXER: Conversar com o Kernel para novos eventos.
        // Se houver tarefas nas filas, o poll tem timeout = 0 (apenas espia e volta).
        // Se estivermos ociosos, podemos dormir até o próximo frame de renderização (16ms).
        let time_to_next_frame = 16_u64.saturating_sub(elapsed);

        let timeout = if self.macrotasks.is_empty() && self.microtasks.is_empty() {
            Some(time_to_next_frame)
        } else {
            Some(0)
        };

        // Bloqueia a thread usando chamadas eficientes do SO (epoll/iocp)
        self.multiplexer.poll(timeout)?;

        Ok(())
    }

    /// Roda o EventLoop continuamente até que o sinal `exit` seja recebido.
    pub fn run(&mut self) -> crate::AceResult<()> {
        while !self.should_exit {
            self.run_once()?;
        }
        Ok(())
    }
}
