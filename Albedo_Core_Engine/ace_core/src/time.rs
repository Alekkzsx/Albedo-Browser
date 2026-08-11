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
//! em uma fonte de tempo centralizada. Em cenários de produção, mapeia para o tempo
//! real do sistema operacional. Em testes de unidade e integração, mapeia para
//! o `MockClock` para execução rápida e determinística (sem sleep() real).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// ----------------------------------------------------------------------------
// Deterministic Mock Clock
// ----------------------------------------------------------------------------

/// O Relógio Virtual Global. Permite aos testes assumirem controle total da
/// progressão temporal do navegador.
pub struct MockClock;

static MOCK_TIME_MS: AtomicU64 = AtomicU64::new(0);

impl MockClock {
    /// Inicializa ou reseta o relógio virtual para 0.
    pub fn reset() {
        MOCK_TIME_MS.store(0, Ordering::SeqCst);
    }

    /// Avança o relógio virtual por um determinado número de milissegundos.
    pub fn advance(ms: u64) {
        MOCK_TIME_MS.fetch_add(ms, Ordering::SeqCst);
    }

    /// Retorna o carimbo de tempo atual em milissegundos a partir da época virtual 0.
    pub fn now_ms() -> u64 {
        MOCK_TIME_MS.load(Ordering::SeqCst)
    }

    /// Simula o avanço de um único frame (~16.6 ms -> arredondado para 16 ou 17).
    /// Convenção para testes: 16 ms.
    pub fn tick_frame() {
        Self::advance(16);
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
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

/// Provedor determinístico que espelha o estado do `MockClock`.
pub struct VirtualTime;

impl TimeProvider for VirtualTime {
    fn now_ms(&self) -> u64 {
        MockClock::now_ms()
    }
}
