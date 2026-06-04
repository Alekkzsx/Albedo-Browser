use once_cell::sync::Lazy;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

static START_INSTANT: Lazy<Instant> = Lazy::new(Instant::now);

/// Retorna um carimbo de data/hora de alta resolução em milissegundos relativo à inicialização do motor.
/// Utiliza o relógio monotônico do sistema para evitar regressões temporais.
pub fn monotonic_now() -> f64 {
    START_INSTANT.elapsed().as_secs_f64() * 1000.0
}

/// Retorna o tempo Unix UTC em milissegundos.
pub fn unix_timestamp_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
