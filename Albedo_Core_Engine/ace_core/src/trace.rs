// ============================================================================
// Albedo Core Engine (ACE)
// File: trace.rs
// Description: Infraestrutura de rastreamento de performance no padrão
//              Trace Event Format (Chrome Tracing).
// Author: Albedo Browser Engineering Team
// ============================================================================

//! # Motor de Tracing Nível Enterprise
//!
//! Esta infraestrutura provê perfilamento sem depender da biblioteca `tracing`.
//! Ao utilizar RAII (Drop Trait), o `TraceSpan` coleta `timestamps` da criação e
//! destruição (saída do escopo) e exporta em formato JSON compatível com
//! `chrome://tracing`.

use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

/// Obtém o timestamp atual em microssegundos absolutos.
#[inline]
fn current_micros() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System clock was before 1970")
        .as_micros() as u64
}

/// Span temporal baseado em RAII (Zero-cost abstrato até o Drop).
pub struct TraceSpan {
    name: &'static str,
}

impl TraceSpan {
    /// Inicializa a captura de tempo no bloco (Fase "B" do Chrome Tracing).
    #[inline]
    pub fn new(name: &'static str) -> Self {
        let ts = current_micros();
        let tid = thread::current().id();

        // Emite o evento "B" (Begin)
        // Como não temos acesso a IDs inteiros de thread nativos em std puro de
        // forma portável rápida (sem libc diretos), formatamos via debug.
        crate::ace_trace!(
            r#"{{"name": "{}", "ph": "B", "ts": {}, "pid": 1, "tid": "{:?}"}},"#,
            name,
            ts,
            tid
        );

        Self { name }
    }
}

impl Drop for TraceSpan {
    /// Dispara no fim do escopo, garantindo o tempo de finalização exato (Fase "E").
    #[inline]
    fn drop(&mut self) {
        let ts = current_micros();
        let tid = thread::current().id();

        // Emite o evento "E" (End)
        crate::ace_trace!(
            r#"{{"name": "{}", "ph": "E", "ts": {}, "pid": 1, "tid": "{:?}"}},"#,
            self.name,
            ts,
            tid
        );
    }
}

// ----------------------------------------------------------------------------
// Public Macros
// ----------------------------------------------------------------------------

/// Inicia um span de rastreamento no escopo atual. Ele se encerra automaticamente
/// no fim do bloco, sem custos de branches ou closures.
#[macro_export]
macro_rules! ace_span {
    ($name:expr) => {
        let _span = $crate::trace::TraceSpan::new($name);
    };
}
