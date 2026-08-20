//! # Triple Buffer Lock-Free (120 FPS Frame Synchronization)
//!
//! Sincronização atômica SP-SC (*Single Producer - Single Consumer*) sem espera e sem mutexes
//! para desacoplamento absoluto entre a thread de Renderização/Layout e a thread da GPU/Compositor.
//!
//! Utiliza máquina de estados atômica de 3 estados com `AtomicU8` e `UnsafeCell` para garantir
//! zero alocação dinâmica e acesso por referência direta (Zero-Copy) sem clone a 120 FPS.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

/// Máscara para extrair o índice do buffer compartilhado (bits 0-1).
const INDEX_MASK: u8 = 0b0000_0011;
/// Bit indicador de novo frame publicado pronto para consumo (bit 2).
const DIRTY_BIT: u8 = 0b0000_0100;

struct SharedTripleBuffer<T> {
    slots: [UnsafeCell<T>; 3],
    state: AtomicU8,
}

unsafe impl<T: Send> Send for SharedTripleBuffer<T> {}
unsafe impl<T: Send> Sync for SharedTripleBuffer<T> {}

/// Produtor do Triple Buffer (executa na thread de renderização / layout pass).
///
/// A escrita e a publicação de novos frames operam em tempo constante $O(1)$ e são Wait-Free.
pub struct TripleBufferProducer<T> {
    shared: Arc<SharedTripleBuffer<T>>,
    back_idx: usize,
}

/// Consumidor do Triple Buffer (executa na thread da GPU / Compositor).
///
/// O consumo opera em tempo $O(1)$ Lock-Free através de CAS atômico e fornece
/// acesso Zero-Copy direto por referência imutável (`&T`).
pub struct TripleBufferConsumer<T> {
    shared: Arc<SharedTripleBuffer<T>>,
    front_idx: usize,
}

/// Aliases convenientes para os tipos de ponta SP-SC.
pub type Producer<T> = TripleBufferProducer<T>;
pub type Consumer<T> = TripleBufferConsumer<T>;

unsafe impl<T: Send> Send for TripleBufferProducer<T> {}
unsafe impl<T: Send> Send for TripleBufferConsumer<T> {}

/// Construtor de conveniência para instâncias de `TripleBuffer`.
pub struct TripleBuffer<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: Clone> TripleBuffer<T> {
    /// Cria um novo par `(Producer, Consumer)` inicializado com clones de `initial`.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(initial: T) -> (TripleBufferProducer<T>, TripleBufferConsumer<T>) {
        triple_buffer(initial)
    }
}

impl<T> TripleBuffer<T> {
    /// Cria um novo par `(Producer, Consumer)` utilizando uma fábrica para inicializar os 3 slots.
    pub fn with_factory<F>(init: F) -> (TripleBufferProducer<T>, TripleBufferConsumer<T>)
    where
        F: FnMut() -> T,
    {
        triple_buffer_with(init)
    }
}

/// Cria um novo par `(Producer, Consumer)` de Triple Buffer inicializado com 3 clones do valor inicial.
pub fn triple_buffer<T: Clone>(initial: T) -> (TripleBufferProducer<T>, TripleBufferConsumer<T>) {
    triple_buffer_with(|| initial.clone())
}

/// Cria um novo par `(Producer, Consumer)` de Triple Buffer chamando uma função geradora para cada slot.
pub fn triple_buffer_with<T, F>(mut init: F) -> (TripleBufferProducer<T>, TripleBufferConsumer<T>)
where
    F: FnMut() -> T,
{
    let shared = Arc::new(SharedTripleBuffer {
        slots: [
            UnsafeCell::new(init()),
            UnsafeCell::new(init()),
            UnsafeCell::new(init()),
        ],
        state: AtomicU8::new(1), // Shared buffer = 1, limpo (DIRTY_BIT = 0)
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
    /// Retorna uma referência mutável direta ao buffer traseiro (*Back Buffer*).
    #[inline]
    pub fn back_mut(&mut self) -> &mut T {
        unsafe { &mut *self.shared.slots[self.back_idx].get() }
    }

    /// Retorna uma referência imutável ao buffer traseiro (*Back Buffer*).
    #[inline]
    pub fn back(&self) -> &T {
        unsafe { &*self.shared.slots[self.back_idx].get() }
    }

    /// Escreve e atualiza o buffer traseiro (*Back Buffer*) através de uma closure mutável.
    #[inline]
    pub fn write_with<F>(&mut self, f: F)
    where
        F: FnOnce(&mut T),
    {
        f(self.back_mut());
    }

    /// Substitui o conteúdo do buffer traseiro diretamente.
    #[inline]
    pub fn write(&mut self, value: T) {
        *self.back_mut() = value;
    }

    /// Publica o frame renderizado atomicamente para o consumidor em tempo constante $O(1)$ Wait-Free.
    #[inline]
    pub fn publish(&mut self) {
        let new_state = (self.back_idx as u8) | DIRTY_BIT;
        let prev_state = self.shared.state.swap(new_state, Ordering::AcqRel);
        self.back_idx = (prev_state & INDEX_MASK) as usize;
    }
}

impl<T> TripleBufferConsumer<T> {
    /// Atualiza o buffer frontal (*Front Buffer*) se um novo frame foi publicado.
    ///
    /// Retorna `true` se houve atualização (novo frame pronto) e `false` caso contrário.
    #[inline]
    pub fn update(&mut self) -> bool {
        let mut current = self.shared.state.load(Ordering::Acquire);
        loop {
            if (current & DIRTY_BIT) == 0 {
                return false;
            }

            let shared_idx = (current & INDEX_MASK) as usize;
            let next_state = self.front_idx as u8; // Limpo (DIRTY_BIT = 0)

            match self.shared.state.compare_exchange_weak(
                current,
                next_state,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    self.front_idx = shared_idx;
                    return true;
                }
                Err(actual) => {
                    current = actual;
                }
            }
        }
    }

    /// Consome o frame mais recente publicado de forma Zero-Copy.
    /// Retorna `Some(&T)` se um novo frame foi recebido, ou `None` se nada mudou desde a última leitura.
    #[inline]
    pub fn consume(&mut self) -> Option<&T> {
        if self.update() {
            Some(self.read())
        } else {
            None
        }
    }

    /// Alias para `consume()`, retornando `Some(&T)` se houver novo frame.
    #[inline]
    pub fn updated(&mut self) -> Option<&T> {
        self.consume()
    }

    /// Retorna uma referência direta ao frame atualmente retido pelo consumidor no front buffer.
    #[inline]
    pub fn read(&self) -> &T {
        unsafe { &*self.shared.slots[self.front_idx].get() }
    }

    /// Retorna uma referência direta ao frame mais recente atualmente retido no front buffer.
    #[inline]
    pub fn read_latest(&self) -> &T {
        self.read()
    }
}

impl<T: Clone> TripleBufferConsumer<T> {
    /// Consome e clona o frame mais recente publicado se houver novidade.
    #[inline]
    pub fn consume_cloned(&mut self) -> Option<T> {
        self.consume().cloned()
    }

    /// Retorna uma cópia clonada do frame mais recente retido no front buffer.
    #[inline]
    pub fn read_latest_cloned(&self) -> T {
        self.read().clone()
    }
}
