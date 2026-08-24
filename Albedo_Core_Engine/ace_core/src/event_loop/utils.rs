//! # Utilitários de Temporização e Agendamento do Event Loop
//!
//! Funções auxiliares para conversão de taxas de quadros (FPS), cálculo de prazos e intervalos de VSync.

use std::time::Duration;

/// Converte uma taxa de quadros por segundo (ex: 60 FPS ou 120 FPS) no intervalo de tempo de cada frame.
#[inline]
pub fn fps_to_interval(fps: u32) -> Duration {
    if fps == 0 {
        Duration::from_millis(16) // Fallback para ~60 FPS
    } else {
        Duration::from_nanos(1_000_000_000 / fps as u64)
    }
}

/// Converte milissegundos inteiros em um `Duration`.
#[inline]
pub const fn ms_to_duration(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

/// Converte um `Duration` na contagem total de milissegundos truncada.
#[inline]
pub fn duration_to_ms(d: Duration) -> u64 {
    d.as_millis() as u64
}

/// Calcula o timestamp absoluto de prazo (`deadline`) somando um timeout ao timestamp inicial em milissegundos.
#[inline]
pub fn compute_deadline(start_ms: u64, timeout: Duration) -> u64 {
    start_ms.saturating_add(timeout.as_millis() as u64)
}

/// Verifica se o timestamp atual ultrapassou o prazo limite (`deadline`).
#[inline]
pub const fn is_deadline_passed(now_ms: u64, deadline_ms: u64) -> bool {
    now_ms >= deadline_ms
}
