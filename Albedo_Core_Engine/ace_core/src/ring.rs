// ============================================================================
// Albedo Core Engine (ACE)
// File: ring.rs
// Description: SPSC Wait-Free Ring Buffer (Fila Circular Lock-Free).
//              Máxima taxa de transferência de I/O entre Threads.
// Author: Albedo Browser Engineering Team
// ============================================================================

//! # Wait-Free SPSC Queue
//!
//! Usado estritamente para comunicação de alta performance entre o Thread
//! de Rede (Producer) e o Event Loop/Parser (Consumer). 
//! A arquitetura aplica ordenação de memória estrita do Hardware (`Acquire`/`Release`)
//! e mitigação de `False Sharing` via alinhamento de 64-bytes (Cache Line).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;

/// Padding para 64 bytes para evitar que as Caches L1/L2 dos cores da CPU fiquem 
/// invalidando umas às outras (False Sharing Ping-Pong).
#[repr(align(64))]
struct CachePadded<T> {
    value: T,
}

pub struct RingBuffer<T> {
    buffer: Box<[UnsafeCell<MaybeUninit<T>>]>,
    capacity: usize,
    /// Produtor (quem insere dados)
    head: CachePadded<AtomicUsize>,
    /// Consumidor (quem retira dados)
    tail: CachePadded<AtomicUsize>,
}

// O Ring Buffer é seguro para transitar entre Threads porque os Atômicos protegem as pontas
// SAFETY: Produtores e Consumidores operam em pontas diferentes. Os ponteiros Head e Tail são 
// atômicos com garantias de Acquire/Release, protegendo as gravações/leituras de Data Races.
unsafe impl<T: Send> Send for RingBuffer<T> {}
unsafe impl<T: Send> Sync for RingBuffer<T> {}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        // Capacidade real + 1 slot vazio para distinguir Cheio de Vazio
        let real_capacity = capacity + 1;
        
        let mut vec = Vec::with_capacity(real_capacity);
        for _ in 0..real_capacity {
            vec.push(UnsafeCell::new(MaybeUninit::uninit()));
        }
        
        Self {
            buffer: vec.into_boxed_slice(),
            capacity: real_capacity,
            head: CachePadded { value: AtomicUsize::new(0) },
            tail: CachePadded { value: AtomicUsize::new(0) },
        }
    }

    /// Operação Wait-Free (nunca bloqueia ou faz loops).
    pub fn push(&self, value: T) -> Result<(), T> {
        let current_head = self.head.value.load(Ordering::Relaxed);
        let current_tail = self.tail.value.load(Ordering::Acquire); // Vê onde o Consumer está
        
        let next_head = (current_head + 1) % self.capacity;
        
        // Se `next_head == tail`, a fila está cheia
        if next_head == current_tail {
            return Err(value);
        }

        // SAFETY: Confirmamos que head != tail no momento lógico atual. A memória no slot
        // `current_head` pertence exclusivamente ao produtor antes da publicação no atômico.
        unsafe {
            // Grava o valor no ponteiro inseguro da célula
            let slot = self.buffer[current_head].get();
            (*slot).as_mut_ptr().write(value);
        }

        // Publica a alteração (Release), garantindo que a memória gravada no bloco 
        // acima esteja fisicamente visível para o Consumidor ANTES do head ser atualizado.
        self.head.value.store(next_head, Ordering::Release);
        
        Ok(())
    }

    /// Operação Wait-Free (nunca bloqueia ou faz loops).
    pub fn pop(&self) -> Option<T> {
        let current_tail = self.tail.value.load(Ordering::Relaxed);
        let current_head = self.head.value.load(Ordering::Acquire); // Vê onde o Producer está
        
        // Se `tail == head`, a fila está vazia
        if current_tail == current_head {
            return None;
        }

        // SAFETY: Confirmamos que a fila não está vazia. O produtor já publicou (Release)
        // e nós já adquirimos (Acquire), tornando a leitura da memória totalmente sincronizada e segura.
        let value = unsafe {
            let slot = self.buffer[current_tail].get();
            (*slot).as_ptr().read()
        };

        let next_tail = (current_tail + 1) % self.capacity;
        
        // Avisa ao Producer que consumimos (Release)
        self.tail.value.store(next_tail, Ordering::Release);

        Some(value)
    }
}

impl<T> Drop for RingBuffer<T> {
    fn drop(&mut self) {
        // Limpar qualquer item que restou não-consumido na fila
        while let Some(item) = self.pop() {
            drop(item);
        }
    }
}


