//! # Notificação de Chamadas Bloqueantes de I/O (Chromium ScopedBlockingCall Pattern)
//!
//! Quando uma thread de um pool de trabalho entra em uma chamada de sistema operacional síncrona
//! bloqueante (leitura/escrita de disco, locks de kernel, IPC síncrono), o pool de threads precisa
//! ser informado para evitar esgotamento de capacidade de processamento (*thread starvation*).
//!
//! O `ScopedBlockingCall` ajusta a contagem de threads ativas dinamicamente enquanto o escopo RAII durar.

use std::sync::atomic::{AtomicUsize, Ordering};

static ACTIVE_BLOCKING_CALLS: AtomicUsize = AtomicUsize::new(0);

/// Tipo de expectativa de bloqueio da chamada de sistema operacional.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockingType {
    /// Pode bloquear, mas geralmente retorna rapidamente (ex: I/O de disco com cache do SO).
    MayBlock,
    /// Certamente bloqueará por período prolongado (ex: sincronização de banco de dados SQLite em disco).
    WillBlock,
}

/// Guarda RAII que notifica o sistema de concorrência sobre o início e fim de um bloqueio de I/O.
pub struct ScopedBlockingCall {
    blocking_type: BlockingType,
}

impl ScopedBlockingCall {
    /// Inicia uma declaração de chamada bloqueante no escopo atual.
    #[inline]
    pub fn new(blocking_type: BlockingType) -> Self {
        ACTIVE_BLOCKING_CALLS.fetch_add(1, Ordering::Relaxed);
        Self { blocking_type }
    }

    /// Retorna o tipo de bloqueio configurado.
    #[inline]
    pub fn blocking_type(&self) -> BlockingType {
        self.blocking_type
    }

    /// Retorna o número total de chamadas bloqueantes ativas no momento em todo o motor.
    #[inline]
    pub fn total_active_blocking_calls() -> usize {
        ACTIVE_BLOCKING_CALLS.load(Ordering::Relaxed)
    }
}

impl Drop for ScopedBlockingCall {
    #[inline]
    fn drop(&mut self) {
        ACTIVE_BLOCKING_CALLS.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scoped_blocking_call_lifecycle() {
        let before = ScopedBlockingCall::total_active_blocking_calls();
        {
            let _blocking = ScopedBlockingCall::new(BlockingType::WillBlock);
            assert_eq!(
                ScopedBlockingCall::total_active_blocking_calls(),
                before + 1
            );
        }
        assert_eq!(ScopedBlockingCall::total_active_blocking_calls(), before);
    }
}
