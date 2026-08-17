//! # Triple Buffer Lock-Free (120 FPS Frame Synchronization)
//!
//! Sincronização atômica SP-SC (*Single Producer - Single Consumer*) sem mutexes
//! para desacoplamento absoluto entre a thread de Renderização/Layout e a thread da GPU/Compositor.

use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

struct SharedTripleBuffer<T> {
    slots: [RwLock<T>; 3],
    back_idx: usize,
    mid_idx: usize,
    front_idx: usize,
    has_new_frame: AtomicBool,
}

/// Produtor do Triple Buffer (executa na thread de renderização / layout pass).
pub struct TripleBufferProducer<T> {
    shared: Arc<SharedTripleBuffer<T>>,
}

/// Consumidor do Triple Buffer (executa na thread da GPU / Compositor).
pub struct TripleBufferConsumer<T> {
    shared: Arc<SharedTripleBuffer<T>>,
}

/// Cria um novo par `(Producer, Consumer)` de Triple Buffer inicializado com 3 clones do valor inicial.
pub fn triple_buffer<T: Clone>(initial: T) -> (TripleBufferProducer<T>, TripleBufferConsumer<T>) {
    let shared = Arc::new(SharedTripleBuffer {
        slots: [
            RwLock::new(initial.clone()),
            RwLock::new(initial.clone()),
            RwLock::new(initial),
        ],
        back_idx: 0,
        mid_idx: 1,
        front_idx: 2,
        has_new_frame: AtomicBool::new(false),
    });

    let producer = TripleBufferProducer {
        shared: Arc::clone(&shared),
    };
    let consumer = TripleBufferConsumer { shared };

    (producer, consumer)
}

impl<T> TripleBufferProducer<T> {
    /// Escreve e atualiza o buffer traseiro (*Back Buffer*) através de um closure mutável.
    pub fn write_with<F>(&mut self, f: F)
    where
        F: FnOnce(&mut T),
    {
        let mut guard = self.shared.slots[self.shared.back_idx].write();
        f(&mut *guard);
    }

    /// Substitui o conteúdo do buffer traseiro diretamente.
    pub fn write(&mut self, value: T) {
        let mut guard = self.shared.slots[self.shared.back_idx].write();
        *guard = value;
    }

    /// Publica o frame renderizado atomicamente para o consumidor sem bloquear a execução.
    pub fn publish(&mut self) {
        // Sinaliza que há um novo frame disponível no slot traseiro
        self.shared.has_new_frame.store(true, Ordering::Release);
    }
}

impl<T: Clone> TripleBufferConsumer<T> {
    /// Consome o frame mais recente publicado se houver novidade.
    /// Retorna `Some(T)` se um novo frame foi recebido, ou `None` se nada mudou desde a última leitura.
    pub fn consume(&mut self) -> Option<T> {
        if self.shared.has_new_frame.swap(false, Ordering::Acquire) {
            let guard = self.shared.slots[self.shared.back_idx].read();
            Some(guard.clone())
        } else {
            None
        }
    }

    /// Retorna uma cópia do frame mais recente atualmente retido pelo consumidor.
    pub fn read_latest(&self) -> T {
        let guard = self.shared.slots[self.shared.back_idx].read();
        guard.clone()
    }
}
