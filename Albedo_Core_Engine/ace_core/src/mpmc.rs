// ============================================================================
// Albedo Core Engine (ACE)
// File: mpmc.rs
// Description: Multi-Producer Multi-Consumer (MPMC) Queue Lock-Free.
//              Implementação baseada no algoritmo de Bounded MPMC de Dmitry Vyukov.
//              Essencial para agendamento de tarefas sem gargalos de Mutex.
// Author: Albedo Browser Engineering Team
// ============================================================================

use core::hint::spin_loop;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Slot individual do buffer da fila.
struct Slot<T> {
    sequence: AtomicUsize,
    data: UnsafeCell<MaybeUninit<T>>,
}

/// Fila Lock-Free MPMC (Multi-Producer Multi-Consumer) limitada (Bounded).
/// Oferece performance extrema para transferência de mensagens e tarefas entre threads.
#[repr(align(64))]
pub struct ArrayQueue<T> {
    buffer: Box<[Slot<T>]>,
    mask: usize,
    head: AtomicUsize,
    // Previne False Sharing forçando o tail a residir em uma linha de cache (64 bytes)
    // diferente da cabeça, já que Produtores mexem no tail e Consumidores no head.
    _pad: [u8; 64],
    tail: AtomicUsize,
}

unsafe impl<T: Send> Send for ArrayQueue<T> {}
unsafe impl<T: Send> Sync for ArrayQueue<T> {}

impl<T> ArrayQueue<T> {
    /// Cria uma nova fila. A capacidade será ajustada para a próxima potência de 2.
    pub fn new(mut capacity: usize) -> Self {
        if capacity < 2 {
            capacity = 2;
        }
        let power_of_two_capacity = capacity.next_power_of_two();

        let mut buffer = Vec::with_capacity(power_of_two_capacity);
        for i in 0..power_of_two_capacity {
            buffer.push(Slot {
                sequence: AtomicUsize::new(i),
                data: UnsafeCell::new(MaybeUninit::uninit()),
            });
        }

        Self {
            buffer: buffer.into_boxed_slice(),
            mask: power_of_two_capacity - 1,
            head: AtomicUsize::new(0),
            _pad: [0; 64],
            tail: AtomicUsize::new(0),
        }
    }

    /// Tenta inserir um elemento na fila.
    /// Retorna `Err(value)` se a fila estiver cheia.
    pub fn push(&self, value: T) -> Result<(), T> {
        let mut tail = self.tail.load(Ordering::Relaxed);
        loop {
            let slot = &self.buffer[tail & self.mask];
            let seq = slot.sequence.load(Ordering::Acquire);
            let diff = seq as isize - tail as isize;

            if diff == 0 {
                // Slot está livre e pronto para receber o elemento da cauda atual.
                if self
                    .tail
                    .compare_exchange_weak(tail, tail + 1, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    // Nós reservamos o slot, agora escrevemos o dado.
                    unsafe {
                        slot.data.get().write(MaybeUninit::new(value));
                    }
                    // Atualiza a sequência para liberar para o consumidor.
                    slot.sequence.store(tail + 1, Ordering::Release);
                    return Ok(());
                }
            } else if diff < 0 {
                // A fila está cheia.
                return Err(value);
            } else {
                // Outra thread avançou a cauda, recarrega e tenta novamente.
                tail = self.tail.load(Ordering::Relaxed);
                spin_loop();
            }
        }
    }

    /// Tenta remover um elemento da fila.
    /// Retorna `None` se a fila estiver vazia.
    pub fn pop(&self) -> Option<T> {
        let mut head = self.head.load(Ordering::Relaxed);
        loop {
            let slot = &self.buffer[head & self.mask];
            let seq = slot.sequence.load(Ordering::Acquire);
            let diff = seq as isize - (head + 1) as isize;

            if diff == 0 {
                // O slot contém dados prontos para a cabeça atual.
                if self
                    .head
                    .compare_exchange_weak(head, head + 1, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    // Nós reservamos o slot, agora lemos o dado.
                    let value = unsafe { slot.data.get().read().assume_init() };
                    // Atualiza a sequência para liberar para um novo produtor (wrap around).
                    slot.sequence.store(head + self.mask + 1, Ordering::Release);
                    return Some(value);
                }
            } else if diff < 0 {
                // A fila está vazia.
                return None;
            } else {
                // Outra thread avançou a cabeça, recarrega e tenta novamente.
                head = self.head.load(Ordering::Relaxed);
                spin_loop();
            }
        }
    }
}
