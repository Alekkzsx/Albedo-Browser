use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Métricas atômicas para observabilidade do pipeline de rede.
#[derive(Debug, Default)]
pub struct FetcherMetrics {
    // Requesições globais
    pub total_requests: AtomicU32,
    pub in_flight_requests: AtomicU32,
    pub failed_requests: AtomicU32,
    pub cancelled_requests: AtomicU32,
    pub timed_out_requests: AtomicU32,

    // Cache
    pub cache_hits: AtomicU32,
    pub cache_misses: AtomicU32,
    pub cache_revalidations: AtomicU32,
    pub cache_evictions: AtomicU32,

    // Transporte e Transferência
    pub bytes_transferred: AtomicU64,
    pub bytes_served_from_cache: AtomicU64,

    // Conexões
    pub connections_created: AtomicU32,
    pub connections_closed: AtomicU32,
    pub active_connections: AtomicU32,
}

impl FetcherMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inc_total_requests(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_in_flight(&self) {
        self.in_flight_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec_in_flight(&self) {
        self.in_flight_requests.fetch_sub(1, Ordering::Relaxed);
    }
}
