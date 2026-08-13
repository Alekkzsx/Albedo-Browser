// ============================================================================
// Albedo Core Engine (ACE)
// File: deque.rs
// Description: Chase-Lev Lock-Free Work-Stealing Deque (Fixed Size).
//              A fila mais rápida possível para o ThreadPool do Albedo.
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::ptr;
use std::sync::atomic::{AtomicIsize, AtomicPtr, Ordering};

/// Capacidade fixa da fila (deve ser uma potência de 2).
const CAPACITY: isize = 4096;
const MASK: isize = CAPACITY - 1;

/// Uma fila Lock-Free Single-Producer, Multiple-Consumer (SPMC) baseada no
/// algoritmo de Chase & Lev (2005) adaptado para modelos fracos de memória.
pub struct WorkerDeque<T> {
    /// O dono da fila escreve/lê daqui
    bottom: AtomicIsize,
    /// Ladrões (thieves) leem daqui
    top: AtomicIsize,
    /// O buffer fixo de ponteiros brutos
    buffer: Box<[AtomicPtr<T>]>,
}

// SAFETY: O design Chase-Lev provê barreira atômica (Acquire/Release) para acesso concorrente seguro,
// mitigando data races internamente, logo é seguro enviar/compartilhar entre Threads.
unsafe impl<T: Send> Send for WorkerDeque<T> {}
unsafe impl<T: Send> Sync for WorkerDeque<T> {}

impl<T> WorkerDeque<T> {
    pub fn new() -> Self {
        let mut vec = Vec::with_capacity(CAPACITY as usize);
        for _ in 0..CAPACITY {
            vec.push(AtomicPtr::new(ptr::null_mut()));
        }

        Self {
            bottom: AtomicIsize::new(0),
            top: AtomicIsize::new(0),
            buffer: vec.into_boxed_slice(),
        }
    }

    /// Operação LIFO. Somente a thread dona (Worker) pode chamar isso.
    pub fn push(&self, task: T) -> Result<(), T> {
        let b = self.bottom.load(Ordering::Relaxed);
        let t = self.top.load(Ordering::Acquire);

        // Verifica se a fila está cheia
        if b - t >= CAPACITY {
            return Err(task);
        }

        let ptr = Box::into_raw(Box::new(task));

        // Armazena no buffer na posição (b % CAPACITY)
        self.buffer[(b & MASK) as usize].store(ptr, Ordering::Relaxed);

        // Publica a atualização do bottom com Release semantics.
        self.bottom.store(b + 1, Ordering::Release);

        Ok(())
    }

    /// Operação LIFO. Somente a thread dona pode chamar isso.
    pub fn pop(&self) -> Option<T> {
        let b = self.bottom.load(Ordering::Relaxed) - 1;
        self.bottom.store(b, Ordering::Relaxed);

        std::sync::atomic::fence(Ordering::SeqCst);

        let t = self.top.load(Ordering::Relaxed);

        if t <= b {
            // A fila tem elementos
            let ptr = self.buffer[(b & MASK) as usize].load(Ordering::Relaxed);

            if t == b {
                // Último elemento, precisamos resolver conflito potencial com ladrões
                let res = self
                    .top
                    .compare_exchange(t, t + 1, Ordering::SeqCst, Ordering::Relaxed);
                self.bottom.store(b + 1, Ordering::Relaxed);

                if res.is_ok() {
                    // Nós ganhamos a disputa contra os ladrões
                    // SAFETY: Ganhamos o Lock via CAS. O ponteiro foi criado de um Box, então podemos recriá-lo.
                    return Some(unsafe { *Box::from_raw(ptr) });
                } else {
                    // Um ladrão roubou o nosso último item!
                    return None;
                }
            } else {
                // Vários elementos, sem disputa
                // SAFETY: Somos o dono exclusivo e não há disputa com ladrões para este índice.
                return Some(unsafe { *Box::from_raw(ptr) });
            }
        } else {
            // A fila estava vazia
            self.bottom.store(b + 1, Ordering::Relaxed);
            None
        }
    }

    /// Operação FIFO. Múltiplas threads (Thieves) podem chamar isso concorrentemente.
    pub fn steal(&self) -> Option<T> {
        loop {
            let t = self.top.load(Ordering::Acquire);
            std::sync::atomic::fence(Ordering::SeqCst);
            let b = self.bottom.load(Ordering::Acquire);

            if t >= b {
                // Fila vazia
                return None;
            }

            let ptr = self.buffer[(t & MASK) as usize].load(Ordering::Relaxed);

            // Tenta roubar o item empurrando o top para baixo via CAS
            if self
                .top
                .compare_exchange(t, t + 1, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                // Sucesso no roubo!
                // SAFETY: CAS bem sucedido significa que conquistamos a posse deste índice na fila.
                return Some(unsafe { *Box::from_raw(ptr) });
            }
            // Falhou, outro ladrão ganhou. O loop tenta de novo (Wait-Free/Lock-Free retry).
        }
    }
}

impl<T> Default for WorkerDeque<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for WorkerDeque<T> {
    fn drop(&mut self) {
        // Limpar qualquer item restante usando pop local
        while self.pop().is_some() {}
    }
}
