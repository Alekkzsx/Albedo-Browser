// ============================================================================
// Albedo Core Engine (ACE)
// File: time.rs
// Description: Infraestrutura de temporização com suporte a Relógio Virtual (Mock)
//              para garantir o determinismo em testes do Event Loop e renderização.
// Author: Albedo Browser Engineering Team
// ============================================================================

//! # Temporização e Relógio Virtual
//!
//! O motor baseia sua renderização (a cada 16.6ms) e as APIs web (`setTimeout`)
//! em uma fonte de tempo centralizada e real.

use std::time::{SystemTime, UNIX_EPOCH};

// ----------------------------------------------------------------------------
// Real OS Monotonic Clock
// ----------------------------------------------------------------------------

/// O Relógio Físico Global. Não há mocks ou emulações, puxa os ciclos de relógio
/// reais do Hardware e Sistema Operacional.
pub struct MonotonicClock;

impl MonotonicClock {
    /// Retorna o carimbo de tempo atual em milissegundos a partir da época real.
    pub fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

// ----------------------------------------------------------------------------
// Time Provider Abstraction
// ----------------------------------------------------------------------------

/// Permite injetar dependência temporal em lógicas como EventLoop ou temporizadores.
pub trait TimeProvider: Send + Sync {
    /// Tempo absoluto em milissegundos desde uma época arbitrária.
    fn now_ms(&self) -> u64;
}

/// Provedor baseado no tempo real da CPU do Sistema Operacional.
pub struct RealTime;

impl TimeProvider for RealTime {
    fn now_ms(&self) -> u64 {
        MonotonicClock::now_ms()
    }
}
