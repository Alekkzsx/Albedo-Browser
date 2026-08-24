//! # Notificação de Pressão de Memória (Memory Pressure)
//!
//! Barramento de notificações global para mitigação proativa de escassez de memória RAM,
//! permitindo que caches de imagens, fontes e arenas descartem buffers inativos antes de OOM.

use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

static LISTENER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Níveis de severidade da pressão de memória do sistema operacional ou do processo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum MemoryPressureLevel {
    /// Nenhuma pressão detectada; operação normal.
    #[default]
    None,
    /// Pressão moderada: recomenda-se liberar caches inativos e decodificações temporárias.
    Moderate,
    /// Pressão crítica: liberação agressiva de buffers de abas em segundo plano e coleta forçada.
    Critical,
}

type Callback = Arc<dyn Fn(MemoryPressureLevel) + Send + Sync>;

/// Gerenciador de eventos de pressão de memória.
pub struct MemoryPressureListener {
    current_level: RwLock<MemoryPressureLevel>,
    listeners: RwLock<Vec<(u64, Callback)>>,
}

impl MemoryPressureListener {
    /// Cria uma nova instância do barramento de pressão de memória.
    pub fn new() -> Self {
        Self {
            current_level: RwLock::new(MemoryPressureLevel::None),
            listeners: RwLock::new(Vec::new()),
        }
    }

    /// Registra um novo ouvinte para receber eventos de pressão de memória.
    /// Retorna o ID único para cancelamento posterior (`unregister`).
    pub fn register<F>(&self, callback: F) -> u64
    where
        F: Fn(MemoryPressureLevel) + Send + Sync + 'static,
    {
        let id = LISTENER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut list = self.listeners.write();
        list.push((id, Arc::new(callback)));
        id
    }

    /// Remove um ouvinte previamente registrado.
    pub fn unregister(&self, id: u64) -> bool {
        let mut list = self.listeners.write();
        if let Some(pos) = list.iter().position(|(listener_id, _)| *listener_id == id) {
            list.remove(pos);
            true
        } else {
            false
        }
    }

    /// Dispara a notificação de novo nível de pressão para todos os ouvintes.
    pub fn notify(&self, level: MemoryPressureLevel) {
        *self.current_level.write() = level;

        let snapshot: Vec<Callback> = {
            let list = self.listeners.read();
            list.iter().map(|(_, cb)| Arc::clone(cb)).collect()
        };

        for callback in snapshot {
            callback(level);
        }
    }

    /// Retorna o nível de pressão de memória atual.
    pub fn current_level(&self) -> MemoryPressureLevel {
        *self.current_level.read()
    }
}

impl Default for MemoryPressureListener {
    fn default() -> Self {
        Self::new()
    }
}
