//! # Sistema de Tempo e Mocking (Relógios)
//!
//! O navegador depende criticamente da abstração de tempo (animações 60fps, timers do JS).
//! 
//! Este módulo fornece relógios virtuais. Em produção, ele usa o `std::time::Instant` 
//! (Alta precisão do sistema). Em testes (como os Web Platform Tests), ele usa um `MockClock`,
//! permitindo congelar ou acelerar o tempo deterministicamente, garantindo que testes complexos
//! não sejam instáveis (flaky) devido ao atraso do SO.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Uma interface unificada para acessar o relógio atual.
pub trait Clock: Send + Sync {
    /// Retorna a quantidade de milissegundos desde uma época inicial.
    fn now_ms(&self) -> u64;
}

/// O relógio oficial de produção. Usa `std::time::Instant`.
pub struct MonotonicClock {
    start: Instant,
}

impl MonotonicClock {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MonotonicClock {
    #[inline]
    fn now_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

/// O relógio determinístico para testes e validações no CI.
pub struct MockClock {
    current_time_ms: AtomicU64,
}

impl MockClock {
    pub fn new(start_ms: u64) -> Self {
        Self {
            current_time_ms: AtomicU64::new(start_ms),
        }
    }

    /// Avança o tempo virtual.
    pub fn advance(&self, duration: Duration) {
        self.current_time_ms.fetch_add(duration.as_millis() as u64, Ordering::SeqCst);
    }
}

impl Clock for MockClock {
    #[inline]
    fn now_ms(&self) -> u64 {
        self.current_time_ms.load(Ordering::SeqCst)
    }
}


