//! # Histórico Circular de Eventos (Breadcrumbs Pattern)
//!
//! Rastreamento em buffer circular dos últimos eventos de ciclo de vida e navegação do motor,
//! permitindo reproduzir a sequência de passos que antecedeu uma falha.

use parking_lot::Mutex;
use std::sync::LazyLock;

/// Registro individual de um evento de rastro (Breadcrumb).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreadcrumbEntry {
    pub category: &'static str,
    pub message: String,
    pub timestamp_ms: u64,
}

static GLOBAL_BREADCRUMBS: LazyLock<BreadcrumbBuffer<32>> = LazyLock::new(BreadcrumbBuffer::new);

/// Buffer circular thread-safe para os últimos `CAPACITY` eventos do motor.
pub struct BreadcrumbBuffer<const CAPACITY: usize> {
    entries: Mutex<Vec<BreadcrumbEntry>>,
}

impl<const CAPACITY: usize> BreadcrumbBuffer<CAPACITY> {
    /// Cria um novo buffer de breadcrumbs vazio.
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::with_capacity(CAPACITY)),
        }
    }

    /// Retorna o buffer global padrão de 32 entradas.
    #[inline]
    pub fn global() -> &'static BreadcrumbBuffer<32> {
        &GLOBAL_BREADCRUMBS
    }

    /// Registra um novo evento no histórico circular com timestamp automático.
    pub fn record(&self, category: &'static str, message: impl Into<String>, timestamp_ms: u64) {
        let mut guard = self.entries.lock();
        if guard.len() >= CAPACITY {
            guard.remove(0);
        }
        guard.push(BreadcrumbEntry {
            category,
            message: message.into(),
            timestamp_ms,
        });
    }

    /// Retorna uma cópia ordenada de todos os eventos recentes registrados.
    pub fn snapshot(&self) -> Vec<BreadcrumbEntry> {
        let guard = self.entries.lock();
        guard.clone()
    }

    /// Limpa o buffer de eventos.
    pub fn clear(&self) {
        let mut guard = self.entries.lock();
        guard.clear();
    }

    /// Registra um evento no buffer global padrão.
    #[inline]
    pub fn add(category: &'static str, message: impl Into<String>, timestamp_ms: u64) {
        Self::global().record(category, message, timestamp_ms);
    }
}

impl<const CAPACITY: usize> Default for BreadcrumbBuffer<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}
