//! # Sistema de Tempo e Mocking (Relógios)
//!
//! O navegador depende criticamente da abstração de tempo (animações 120fps, timers do JS).
//!
//! Este módulo fornece relógios virtuais com suporte a alta resolução (W3C High Resolution Time Level 3 / `DOMHighResTimeStamp`).
//! Em produção, ele usa o `std::time::Instant` de alta precisão do sistema operacional.
//! Em testes, ele usa o `MockClock`, permitindo congelar ou avançar o tempo deterministicamente.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Uma interface unificada para acessar o relógio atual com suporte a alta precisão.
pub trait Clock: Send + Sync {
    /// Retorna a quantidade de milissegundos inteiros desde a época inicial.
    fn now_ms(&self) -> u64;

    /// Retorna a quantidade de microssegundos inteiros ($\mu s$) desde a época inicial.
    fn now_us(&self) -> u64;

    /// Retorna a quantidade de nanossegundos ($ns$) desde a época inicial.
    fn now_ns(&self) -> u128;

    /// Retorna a duração decorrida como `Duration`.
    fn now_duration(&self) -> Duration;

    /// Retorna o timestamp em milissegundos com fração decimal em ponto flutuante,
    /// compatível com o padrão W3C `DOMHighResTimeStamp` (`performance.now()`).
    #[inline]
    fn now_highres(&self) -> f64 {
        self.now_us() as f64 / 1000.0
    }
}

/// O relógio oficial de produção. Usa `std::time::Instant` de alta precisão.
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

    #[inline]
    fn now_us(&self) -> u64 {
        self.start.elapsed().as_micros() as u64
    }

    #[inline]
    fn now_ns(&self) -> u128 {
        self.start.elapsed().as_nanos()
    }

    #[inline]
    fn now_duration(&self) -> Duration {
        self.start.elapsed()
    }
}

/// O relógio determinístico para testes e validações no CI (com precisão em microssegundos).
pub struct MockClock {
    current_time_us: AtomicU64,
}

impl MockClock {
    /// Cria um novo relógio simulado a partir de um tempo inicial em milissegundos.
    pub fn new(start_ms: u64) -> Self {
        Self {
            current_time_us: AtomicU64::new(start_ms.saturating_mul(1000)),
        }
    }

    /// Cria um novo relógio simulado a partir de microssegundos.
    pub fn from_micros(start_us: u64) -> Self {
        Self {
            current_time_us: AtomicU64::new(start_us),
        }
    }

    /// Avança o tempo virtual por uma `Duration`.
    pub fn advance(&self, duration: Duration) {
        self.current_time_us
            .fetch_add(duration.as_micros() as u64, Ordering::SeqCst);
    }

    /// Avança o tempo virtual por uma quantidade de milissegundos.
    pub fn advance_millis(&self, ms: u64) {
        self.current_time_us
            .fetch_add(ms.saturating_mul(1000), Ordering::SeqCst);
    }

    /// Avança o tempo virtual por uma quantidade de microssegundos.
    pub fn advance_micros(&self, us: u64) {
        self.current_time_us.fetch_add(us, Ordering::SeqCst);
    }
}

impl Clock for MockClock {
    #[inline]
    fn now_ms(&self) -> u64 {
        self.current_time_us.load(Ordering::SeqCst) / 1000
    }

    #[inline]
    fn now_us(&self) -> u64 {
        self.current_time_us.load(Ordering::SeqCst)
    }

    #[inline]
    fn now_ns(&self) -> u128 {
        (self.current_time_us.load(Ordering::SeqCst) as u128) * 1000
    }

    #[inline]
    fn now_duration(&self) -> Duration {
        Duration::from_micros(self.current_time_us.load(Ordering::SeqCst))
    }
}
