//! # Vetor Híbrido Stack/Heap (Small-Vector Optimization)
//!
//! Armazena até `N` elementos diretamente no corpo da struct (stack ou arena).
//! Se a capacidade inline for excedida, transiciona transparentemente para o heap (`Vec<T>`).
//! Evita 95%+ de alocações dinâmicas para listas de classes CSS, atributos DOM e nós filhos.

use std::fmt;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};

enum InlineVecStorage<T, const N: usize> {
    Inline {
        len: usize,
        data: [MaybeUninit<T>; N],
    },
    Heap(Vec<T>),
}

/// Vetor híbrido com armazenamento local de até `N` elementos antes de alocar no heap.
pub struct InlineVec<T, const N: usize> {
    storage: InlineVecStorage<T, N>,
}

impl<T, const N: usize> InlineVec<T, N> {
    /// Cria um novo `InlineVec` vazio no buffer inline.
    pub const fn new() -> Self {
        Self {
            storage: InlineVecStorage::Inline {
                len: 0,
                data: [const { MaybeUninit::uninit() }; N],
            },
        }
    }

    /// Retorna o número de elementos contidos no vetor.
    #[inline]
    pub fn len(&self) -> usize {
        match &self.storage {
            InlineVecStorage::Inline { len, .. } => *len,
            InlineVecStorage::Heap(vec) => vec.len(),
        }
    }

    /// Retorna `true` se o vetor estiver vazio.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Retorna `true` se os elementos ainda estão armazenados no buffer inline.
    #[inline]
    pub fn is_inline(&self) -> bool {
        matches!(self.storage, InlineVecStorage::Inline { .. })
    }

    /// Adiciona um elemento ao final do vetor.
    pub fn push(&mut self, item: T) {
        match &mut self.storage {
            InlineVecStorage::Inline { len, data } => {
                if *len < N {
                    data[*len].write(item);
                    *len += 1;
                } else {
                    // Transição para o Heap: migra os N elementos inline para um Vec
                    let mut heap_vec = Vec::with_capacity(N * 2 + 1);
                    for slot in data.iter().take(*len) {
                        // SAFETY: Os slots 0..*len foram inicializados
                        let val = unsafe { slot.assume_init_read() };
                        heap_vec.push(val);
                    }
                    heap_vec.push(item);
                    self.storage = InlineVecStorage::Heap(heap_vec);
                }
            }
            InlineVecStorage::Heap(vec) => {
                vec.push(item);
            }
        }
    }

    /// Remove e retorna o último elemento, se houver.
    pub fn pop(&mut self) -> Option<T> {
        match &mut self.storage {
            InlineVecStorage::Inline { len, data } => {
                if *len > 0 {
                    *len -= 1;
                    // SAFETY: O índice *len foi inicializado
                    Some(unsafe { data[*len].assume_init_read() })
                } else {
                    None
                }
            }
            InlineVecStorage::Heap(vec) => vec.pop(),
        }
    }

    /// Retorna uma fatia imutável contendo todos os elementos.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        match &self.storage {
            InlineVecStorage::Inline { len, data } => {
                // SAFETY: Fatias 0..*len estão inicializadas
                unsafe { std::slice::from_raw_parts(data.as_ptr() as *const T, *len) }
            }
            InlineVecStorage::Heap(vec) => vec.as_slice(),
        }
    }

    /// Retorna uma fatia mutável contendo todos os elementos.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match &mut self.storage {
            InlineVecStorage::Inline { len, data } => {
                // SAFETY: Fatias 0..*len estão inicializadas
                unsafe { std::slice::from_raw_parts_mut(data.as_mut_ptr() as *mut T, *len) }
            }
            InlineVecStorage::Heap(vec) => vec.as_mut_slice(),
        }
    }

    /// Limpa o vetor, descartando todos os elementos.
    pub fn clear(&mut self) {
        match &mut self.storage {
            InlineVecStorage::Inline { len, data } => {
                for slot in data.iter_mut().take(*len) {
                    unsafe {
                        slot.assume_init_drop();
                    }
                }
                *len = 0;
            }
            InlineVecStorage::Heap(vec) => {
                vec.clear();
            }
        }
    }
}

impl<T, const N: usize> Drop for InlineVec<T, N> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T: Clone, const N: usize> Clone for InlineVec<T, N> {
    fn clone(&self) -> Self {
        let mut new_vec = Self::new();
        for item in self.as_slice() {
            new_vec.push(item.clone());
        }
        new_vec
    }
}

impl<T: PartialEq, const N: usize> PartialEq for InlineVec<T, N> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Eq, const N: usize> Eq for InlineVec<T, N> {}

impl<T, const N: usize> Default for InlineVec<T, N> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Deref for InlineVec<T, N> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const N: usize> DerefMut for InlineVec<T, N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for InlineVec<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}

impl<T, const N: usize> FromIterator<T> for InlineVec<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut vec = Self::new();
        for item in iter {
            vec.push(item);
        }
        vec
    }
}

impl<T, const N: usize> Extend<T> for InlineVec<T, N> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.push(item);
        }
    }
}

impl<T, const N: usize> From<Vec<T>> for InlineVec<T, N> {
    fn from(vec: Vec<T>) -> Self {
        if vec.len() <= N {
            let mut inline = Self::new();
            for item in vec {
                inline.push(item);
            }
            inline
        } else {
            Self {
                storage: InlineVecStorage::Heap(vec),
            }
        }
    }
}

impl<T, const N: usize> From<[T; N]> for InlineVec<T, N> {
    fn from(arr: [T; N]) -> Self {
        let mut inline = Self::new();
        for item in arr {
            inline.push(item);
        }
        inline
    }
}

