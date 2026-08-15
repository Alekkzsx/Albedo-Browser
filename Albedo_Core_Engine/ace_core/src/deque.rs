// ============================================================================
// Albedo Core Engine (ACE)
// File: deque.rs
// Description: Dynamic Chase-Lev Lock-Free Work-Stealing Deque.
//              Array circular redimensionável dinamicamente com substituição
//              atômica e liberação segura de buffers antigos via EBR.
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::ptr;
use std::sync::atomic::{AtomicIsize, AtomicPtr, Ordering};

const INITIAL_CAPACITY: isize = 256;

/// Buffer circular interno redimensionável.
struct Buffer<T> {
    capacity: isize,
    mask: isize,
    storage: Box<[AtomicPtr<T>]>,
}

impl<T> Buffer<T> {
    fn new(capacity: isize) -> Self {
        let mut vec = Vec::with_capacity(capacity as usize);
        for _ in 0..capacity {
            vec.push(AtomicPtr::new(ptr::null_mut()));
        }
        Self {
            capacity,
            mask: capacity - 1,
            storage: vec.into_boxed_slice(),
        }
    }

    #[inline(always)]
    fn get(&self, index: isize) -> *mut T {
        unsafe {
            self.storage
                .get_unchecked((index & self.mask) as usize)
                .load(Ordering::Relaxed)
        }
    }

    #[inline(always)]
    fn put(&self, index: isize, ptr: *mut T) {
        unsafe {
            self.storage
                .get_unchecked((index & self.mask) as usize)
                .store(ptr, Ordering::Relaxed);
        }
    }
}

/// Fila Lock-Free Single-Producer, Multiple-Consumer (SPMC) com crescimento dinâmico,
/// baseada no algoritmo de Chase & Lev (2005) e modelo de memória C11 (Lê et al., 2013).
pub struct WorkerDeque<T> {
    bottom: AtomicIsize,
    top: AtomicIsize,
    buffer: AtomicPtr<Buffer<T>>,
}

// SAFETY: O design Chase-Lev provê barreiras atômicas (Acquire/Release) para acesso concorrente seguro,
// mitigando data races internamente, logo é seguro enviar/compartilhar entre Threads.
unsafe impl<T: Send> Send for WorkerDeque<T> {}
unsafe impl<T: Send> Sync for WorkerDeque<T> {}

impl<T> WorkerDeque<T> {
    pub fn new() -> Self {
        let initial_buf = Box::new(Buffer::new(INITIAL_CAPACITY));
        Self {
            bottom: AtomicIsize::new(0),
            top: AtomicIsize::new(0),
            buffer: AtomicPtr::new(Box::into_raw(initial_buf)),
        }
    }

    /// Operação LIFO. Somente a thread dona (Worker) pode chamar isso.
    /// Redimensiona dinamicamente a capacidade se a fila estiver cheia.
    #[inline(always)]
    pub fn push(&self, task: T) -> Result<(), T> {
        let b = self.bottom.load(Ordering::Relaxed);
        let t = self.top.load(Ordering::Acquire);
        let mut buf_ptr = self.buffer.load(Ordering::Relaxed);
        let mut buf = unsafe { &*buf_ptr };

        // Verifica se a fila está cheia. Se estiver, redimensiona (cresce 2x)
        if b - t >= buf.capacity {
            let new_cap = buf.capacity * 2;
            let new_buf = Box::new(Buffer::new(new_cap));

            // Copia todos os elementos pendentes para o novo buffer
            for i in t..b {
                new_buf.put(i, buf.get(i));
            }

            let new_buf_ptr = Box::into_raw(new_buf);
            self.buffer.store(new_buf_ptr, Ordering::Release);

            // Agenda a liberação segura do buffer antigo pelo EBR
            crate::ebr::defer_drop(buf_ptr);

            buf_ptr = new_buf_ptr;
            buf = unsafe { &*buf_ptr };
        }

        let ptr = Box::into_raw(Box::new(task));
        buf.put(b, ptr);

        // Publica a atualização do bottom com Release semantics.
        self.bottom.store(b + 1, Ordering::Release);

        Ok(())
    }

    /// Operação LIFO. Somente a thread dona pode chamar isso.
    #[inline(always)]
    pub fn pop(&self) -> Option<T> {
        let b = self.bottom.load(Ordering::Relaxed) - 1;
        self.bottom.store(b, Ordering::Relaxed);

        std::sync::atomic::fence(Ordering::SeqCst);

        let t = self.top.load(Ordering::Relaxed);

        if t <= b {
            let buf = unsafe { &*self.buffer.load(Ordering::Relaxed) };
            let ptr = buf.get(b);

            if t == b {
                // Último elemento, precisamos resolver conflito potencial com ladrões
                let res = self
                    .top
                    .compare_exchange(t, t + 1, Ordering::SeqCst, Ordering::Relaxed);
                self.bottom.store(b + 1, Ordering::Relaxed);

                if res.is_ok() {
                    // Nós ganhamos a disputa contra os ladrões
                    Some(unsafe { *Box::from_raw(ptr) })
                } else {
                    // Um ladrão roubou o nosso último item!
                    None
                }
            } else {
                // Vários elementos, sem disputa
                Some(unsafe { *Box::from_raw(ptr) })
            }
        } else {
            // A fila estava vazia
            self.bottom.store(b + 1, Ordering::Relaxed);
            None
        }
    }

    /// Operação FIFO. Múltiplas threads (Thieves) podem chamar isso concorrentemente.
    #[inline(always)]
    pub fn steal(&self) -> Option<T> {
        let _guard = crate::ebr::Guard::pin();
        loop {
            let t = self.top.load(Ordering::Acquire);
            std::sync::atomic::fence(Ordering::SeqCst);
            let b = self.bottom.load(Ordering::Acquire);

            if t >= b {
                // Fila vazia
                return None;
            }

            let buf = unsafe { &*self.buffer.load(Ordering::Acquire) };
            let ptr = buf.get(t);

            // Tenta roubar o item empurrando o top para baixo via CAS
            if self
                .top
                .compare_exchange(t, t + 1, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                // Sucesso no roubo!
                return Some(unsafe { *Box::from_raw(ptr) });
            }
            // Falhou, outro ladrão ganhou. Tenta novamente.
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

        let buf_ptr = self.buffer.load(Ordering::Relaxed);
        if !buf_ptr.is_null() {
            unsafe {
                let _ = Box::from_raw(buf_ptr);
            }
        }
    }
}
