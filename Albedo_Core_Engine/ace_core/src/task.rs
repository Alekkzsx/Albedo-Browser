//! # Orquestrador de Tarefas Paralelas
//!
//! Envelopa a crate `rayon` para fornecer pools de thread especializados para tarefas
//! intensivas de CPU (como a construção da Render Tree e do CSS Cascade).
//! 
//! O diferencial deste módulo é o sistema de **Catch Panic**: se o layout de uma aba 
//! colapsar (panic), o navegador não morrerá inteiro. Nós capturamos o pânico na thread
//! paralela e o devolvemos como um Erro Tratável ("Aw, Snap!" Page), garantindo
//! a resiliência global do Albedo.

use std::panic::{catch_unwind, AssertUnwindSafe};
use tracing::{error, warn};
use crate::error::AceError;

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
/// 
/// Se a função der panic (ex: out of bounds array durante o layout CSS), 
/// nós evitamos a queda do navegador e retornamos um `AceError::InvalidOperation`.
pub fn spawn_safe<F, R>(f: F) -> Result<R, AceError>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let wrapped_f = AssertUnwindSafe(f);
    
    // catch_unwind previne que o panic escape e destrua a thread atual/processo.
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
            Err(AceError::InvalidOperation(format!("Panic interno capturado: {}", msg)))
        }
    }
}


