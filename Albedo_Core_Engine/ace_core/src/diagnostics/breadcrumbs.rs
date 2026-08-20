//! # Histórico Circular de Eventos (Breadcrumbs Pattern)
//!
//! Rastreamento em buffer circular dos últimos eventos de ciclo de vida e navegação do motor,
//! permitindo reproduzir a sequência de passos que antecedeu uma falha.

use parking_lot::Mutex;
use std::collections::VecDeque;
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
    entries: Mutex<VecDeque<BreadcrumbEntry>>,
}

impl<const CAPACITY: usize> BreadcrumbBuffer<CAPACITY> {
    /// Cria um novo buffer de breadcrumbs vazio.
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(VecDeque::with_capacity(CAPACITY)),
        }
    }

    /// Retorna a capacidade máxima configurada para este buffer.
    #[inline]
    pub fn capacity(&self) -> usize {
        CAPACITY
    }

    /// Retorna o número de entradas atualmente presentes no buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.lock().len()
    }

    /// Retorna `true` se o buffer estiver vazio.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.lock().is_empty()
    }

    /// Retorna o buffer global padrão de 32 entradas.
    #[inline]
    pub fn global() -> &'static BreadcrumbBuffer<32> {
        &GLOBAL_BREADCRUMBS
    }

    /// Registra um novo evento no histórico circular com timestamp.
    ///
    /// Se `CAPACITY == 0`, a operação é uma no-op segura.
    /// A rotação em caso de capacidade cheia opera em tempo $O(1)$ via `pop_front()`.
    pub fn record(&self, category: &'static str, message: impl Into<String>, timestamp_ms: u64) {
        if CAPACITY == 0 {
            return;
        }
        let message = message.into();
        let mut guard = self.entries.lock();
        if guard.len() >= CAPACITY {
            guard.pop_front();
        }
        guard.push_back(BreadcrumbEntry {
            category,
            message,
            timestamp_ms,
        });
    }

    /// Adiciona diretamente um [`BreadcrumbEntry`] ao buffer.
    pub fn push(&self, item: BreadcrumbEntry) {
        if CAPACITY == 0 {
            return;
        }
        let mut guard = self.entries.lock();
        if guard.len() >= CAPACITY {
            guard.pop_front();
        }
        guard.push_back(item);
    }

    /// Retorna uma cópia ordenada de todos os eventos recentes registrados.
    pub fn snapshot(&self) -> Vec<BreadcrumbEntry> {
        let guard = self.entries.lock();
        guard.iter().cloned().collect()
    }

    /// Executa uma closure com as fatias contíguas do `VecDeque` subjacente.
    pub fn with_slices<R>(&self, f: impl FnOnce((&[BreadcrumbEntry], &[BreadcrumbEntry])) -> R) -> R {
        let guard = self.entries.lock();
        f(guard.as_slices())
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
