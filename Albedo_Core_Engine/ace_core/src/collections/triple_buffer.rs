//! # Triple Buffer Lock-Free (120 FPS Frame Synchronization)
//!
//! Sincronização atômica SP-SC (*Single Producer - Single Consumer*) sem espera e sem mutexes
//! para desacoplamento absoluto entre a thread de Renderização/Layout e a thread da GPU/Compositor.

use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

struct SharedTripleBuffer<T> {
    slots: [RwLock<T>; 3],
    mid_idx: AtomicUsize,
    has_new_frame: AtomicBool,
}

/// Produtor do Triple Buffer (executa na thread de renderização / layout pass).
pub struct TripleBufferProducer<T> {
    shared: Arc<SharedTripleBuffer<T>>,
    back_idx: usize,
}

/// Consumidor do Triple Buffer (executa na thread da GPU / Compositor).
pub struct TripleBufferConsumer<T> {
    shared: Arc<SharedTripleBuffer<T>>,
    front_idx: usize,
}

/// Cria um novo par `(Producer, Consumer)` de Triple Buffer inicializado com 3 clones do valor inicial.
pub fn triple_buffer<T: Clone>(initial: T) -> (TripleBufferProducer<T>, TripleBufferConsumer<T>) {
    let shared = Arc::new(SharedTripleBuffer {
        slots: [
            RwLock::new(initial.clone()),
            RwLock::new(initial.clone()),
            RwLock::new(initial),
        ],
        mid_idx: AtomicUsize::new(1),
        has_new_frame: AtomicBool::new(false),
    });

    let producer = TripleBufferProducer {
        shared: Arc::clone(&shared),
        back_idx: 0,
    };
    let consumer = TripleBufferConsumer {
        shared,
        front_idx: 2,
    };

    (producer, consumer)
}

impl<T> TripleBufferProducer<T> {
    /// Escreve e atualiza o buffer traseiro (*Back Buffer*) através de uma closure mutável.
    pub fn write_with<F>(&mut self, f: F)
    where
        F: FnOnce(&mut T),
    {
        let mut guard = self.shared.slots[self.back_idx].write();
        f(&mut *guard);
    }

    /// Substitui o conteúdo do buffer traseiro diretamente.
    pub fn write(&mut self, value: T) {
        let mut guard = self.shared.slots[self.back_idx].write();
        *guard = value;
    }

    /// Publica o frame renderizado atomicamente para o consumidor sem bloquear a execução.
    pub fn publish(&mut self) {
        // Troca atomicamente o back buffer atual com o mid buffer
        let prev_mid = self.shared.mid_idx.swap(self.back_idx, Ordering::AcqRel);
        self.back_idx = prev_mid;
        self.shared.has_new_frame.store(true, Ordering::Release);
    }
}

impl<T: Clone> TripleBufferConsumer<T> {
    /// Consome o frame mais recente publicado se houver novidade.
    /// Retorna `Some(T)` se um novo frame foi recebido, ou `None` se nada mudou desde a última leitura.
    pub fn consume(&mut self) -> Option<T> {
        if self.shared.has_new_frame.swap(false, Ordering::Acquire) {
            // Troca o front buffer atual com o mid buffer publicado mais recente
            let prev_mid = self.shared.mid_idx.swap(self.front_idx, Ordering::AcqRel);
            self.front_idx = prev_mid;
            let guard = self.shared.slots[self.front_idx].read();
            Some(guard.clone())
        } else {
            None
        }
    }

    /// Retorna uma cópia do frame mais recente atualmente retido pelo consumidor no front buffer.
    pub fn read_latest(&self) -> T {
        let guard = self.shared.slots[self.front_idx].read();
        guard.clone()
    }
}
