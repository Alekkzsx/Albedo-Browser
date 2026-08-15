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

use std::time::{Instant, SystemTime, UNIX_EPOCH};

// ----------------------------------------------------------------------------
// Real OS Monotonic Clock (Strict)
// ----------------------------------------------------------------------------

/// O Relógio Monotônico Estrito.
/// Em vez de depender do relógio global da parede (`SystemTime`), usa os ciclos contínuos
/// da máquina física (`Instant`). Imune a NTP drifts e viagens no tempo do SO.
pub struct StrictMonotonicClock;

impl StrictMonotonicClock {
    /// O Instante de inicialização da Engine, usado como âncora absoluta Zero.
    fn epoch() -> Instant {
        static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        *START.get_or_init(Instant::now)
    }

    /// Retorna o tempo em milissegundos desde que a Engine iniciou (Estritamente monotônico).
    pub fn now_ms() -> u64 {
        Self::epoch().elapsed().as_millis() as u64
    }

    /// Retorna o tempo com precisão sub-microsegundo (Nanos) para telemetria bruta.
    pub fn now_ns() -> u64 {
        Self::epoch().elapsed().as_nanos() as u64
    }
}

// ----------------------------------------------------------------------------
// Hardware Cycle Counter (RDTSC)
// ----------------------------------------------------------------------------

/// Relógio baseado diretamente na instrução de ciclo do processador físico.
/// Custo computacional O(0) em syscalls. Vital para micro-benchmarks do layout.
pub struct CycleClock;

impl CycleClock {
    /// Obtém o número imediato de ciclos passados pelo pipeline da CPU atual.
    #[inline(always)]
    pub fn now_ticks() -> u64 {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::x86_64::_rdtsc()
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            // Fallback para outras arquiteturas usando ns brutos
            StrictMonotonicClock::now_ns()
        }
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
        StrictMonotonicClock::now_ms()
    }
}
