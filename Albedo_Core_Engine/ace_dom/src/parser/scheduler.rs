//! # Escalonador de Parsing HTML (Blink HTMLParserScheduler Pattern)
//!
//! Controla fatias de tempo (*time-slicing*) de processamento de chunks na thread principal,
//! garantindo que o Event Loop possa renderizar frames a 60/120 FPS sem sofrer starvation.

use crate::parser::background::BackgroundParserHandle;
use crate::tree_builder::HTMLTreeBuilder;
use std::time::{Duration, Instant};

/// Orçamento de tempo padrão para processamento de tokens por frame (5 milissegundos).
pub const DEFAULT_PARSER_BUDGET: Duration = Duration::from_millis(5);

/// Escalonador de parsing com controle de orçamento de CPU.
#[derive(Debug, Clone)]
pub struct HTMLParserScheduler {
    budget: Duration,
}

impl Default for HTMLParserScheduler {
    fn default() -> Self {
        Self::new(DEFAULT_PARSER_BUDGET)
    }
}

impl HTMLParserScheduler {
    /// Cria um novo escalonador com o orçamento especificado por fatia de execução.
    pub fn new(budget: Duration) -> Self {
        Self { budget }
    }

    /// Executa o bombeamento de tokens até esgotar o orçamento de tempo ou até que não haja mais tokens disponíveis.
    ///
    /// Retorna `(total_tokens_processados, concluiu_parsing)`.
    pub fn pump_with_budget(
        &self,
        handle: &mut BackgroundParserHandle,
        builder: &mut HTMLTreeBuilder,
    ) -> (usize, bool) {
        let start = Instant::now();
        let mut total_tokens = 0;

        while !handle.is_completed() {
            let processed = handle.pump(builder);
            total_tokens += processed;

            if processed == 0 || start.elapsed() >= self.budget {
                break;
            }
        }

        (total_tokens, handle.is_completed())
    }
}
