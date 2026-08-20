//! # Orquestrador de Tarefas Paralelas e Cancelamento
//!
//! Envelopa a crate `rayon` para fornecer pools de thread e provê tokens de cancelamento cooperativo (`CancellationToken`).

pub mod cancellation;
pub mod sequence_checker;

pub use cancellation::CancellationToken;
pub use sequence_checker::SequenceChecker;

use crate::error::AceError;
use std::panic::{catch_unwind, AssertUnwindSafe};
use tracing::{error, warn};

/// Inicializa o pool global de threads de CPU do Albedo.
/// Deve ser chamado uma vez no início (no processo principal do browser).
pub fn init_thread_pool(num_threads: usize) {
    let result = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .thread_name(|i| format!("AceWorker-{}", i))
        .build_global();

    if let Err(e) = result {
        warn!("Tentativa de reinicializar o ThreadPool global: {}", e);
    }
}

/// Executa uma função intensiva em paralelo e intercepta panics catastróficos.
pub fn spawn_safe<F, R>(f: F) -> Result<R, AceError>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let wrapped_f = AssertUnwindSafe(f);

    match catch_unwind(wrapped_f) {
        Ok(result) => Ok(result),
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "Panic de tipo desconhecido".to_string()
            };

            error!("CRÍTICO: Tarefa paralela falhou com PANIC: {}", msg);
            Err(AceError::InvalidOperation(format!(
                "Panic interno capturado: {}",
                msg
            )))
        }
    }
}
