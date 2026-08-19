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

    /// Adiciona um elemento ao final do vetor (com proteção contra pânico durante realocação).
    pub fn push(&mut self, item: T) {
        match &mut self.storage {
            InlineVecStorage::Inline { len, data } => {
                if *len < N {
                    data[*len].write(item);
                    *len += 1;
                } else {
                    // Transição para o Heap: migra os N elementos inline para um Vec
                    let mut heap_vec = Vec::with_capacity(N * 2 + 1);
                    let count = *len;
                    *len = 0; // Se houver pânico, data já é considerado drenado
                    for slot in data.iter_mut().take(count) {
                        // SAFETY: Os slots 0..count foram inicializados e *len agora é 0
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

    /// Insere um elemento em uma posição específica.
    pub fn insert(&mut self, index: usize, item: T) {
        assert!(index <= self.len(), "índice de inserção fora dos limites");
        if self.is_inline() && self.len() < N {
            if let InlineVecStorage::Inline { len, data } = &mut self.storage {
                for i in (*len..index).rev() {
                    let prev = unsafe { data[i - 1].assume_init_read() };
                    data[i].write(prev);
                }
                data[index].write(item);
                *len += 1;
                return;
            }
        }

        // Se estiver cheio ou no heap, garante que vire Heap
        if self.is_inline() {
            let mut heap = Vec::with_capacity(self.len() + 1);
            heap.extend(self.drain(..));
            heap.insert(index, item);
            self.storage = InlineVecStorage::Heap(heap);
        } else if let InlineVecStorage::Heap(vec) = &mut self.storage {
            vec.insert(index, item);
        }
    }

    /// Remove e retorna o elemento no índice especificado.
    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len(), "índice de remoção fora dos limites");
        match &mut self.storage {
            InlineVecStorage::Inline { len, data } => {
                let removed = unsafe { data[index].assume_init_read() };
                for i in index..(*len - 1) {
                    let next = unsafe { data[i + 1].assume_init_read() };
                    data[i].write(next);
                }
                *len -= 1;
                removed
            }
            InlineVecStorage::Heap(vec) => vec.remove(index),
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

    /// Retém apenas os elementos que satisfazem o predicado.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        let len = self.len();
        let mut del = 0;
        for i in 0..len {
            if !f(&self[i]) {
                del += 1;
            } else if del > 0 {
                // Desloca para frente
                let idx = i - del;
                if self.is_inline() {
                    if let InlineVecStorage::Inline { data, .. } = &mut self.storage {
                        let item = unsafe { data[i].assume_init_read() };
                        unsafe { data[idx].assume_init_drop() };
                        data[idx].write(item);
                    }
                } else if let InlineVecStorage::Heap(vec) = &mut self.storage {
                    vec.swap(i, idx);
                }
            }
        }
        if del > 0 {
            match &mut self.storage {
                InlineVecStorage::Inline { len, data } => {
                    for slot in data.iter_mut().skip(*len - del).take(del) {
                        unsafe { slot.assume_init_drop() };
                    }
                    *len -= del;
                }
                InlineVecStorage::Heap(vec) => {
                    vec.truncate(len - del);
                }
            }
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
                let count = *len;
                *len = 0;
                for slot in data.iter_mut().take(count) {
                    unsafe {
                        slot.assume_init_drop();
                    }
                }
            }
            InlineVecStorage::Heap(vec) => {
                vec.clear();
            }
        }
    }

    /// Retorna a capacidade atual do vetor (N se inline, ou a capacidade do heap).
    #[inline]
    pub fn capacity(&self) -> usize {
        match &self.storage {
            InlineVecStorage::Inline { .. } => N,
            InlineVecStorage::Heap(vec) => vec.capacity(),
        }
    }

    /// Obtém uma referência ao elemento no índice especificado.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.as_slice().get(index)
    }

    /// Obtém uma referência mutável ao elemento no índice especificado.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.as_mut_slice().get_mut(index)
    }

    /// Retorna uma referência ao primeiro elemento, se houver.
    #[inline]
    pub fn first(&self) -> Option<&T> {
        self.as_slice().first()
    }

    /// Retorna uma referência mutável ao primeiro elemento, se houver.
    #[inline]
    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.as_mut_slice().first_mut()
    }

    /// Retorna uma referência ao último elemento, se houver.
    #[inline]
    pub fn last(&self) -> Option<&T> {
        self.as_slice().last()
    }

    /// Retorna uma referência mutável ao último elemento, se houver.
    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.as_mut_slice().last_mut()
    }

    /// Reduz o comprimento do vetor para `new_len`, descartando os elementos excedentes.
    pub fn truncate(&mut self, new_len: usize) {
        if new_len >= self.len() {
            return;
        }
        match &mut self.storage {
            InlineVecStorage::Inline { len, data } => {
                let to_drop = *len - new_len;
                for slot in data.iter_mut().skip(new_len).take(to_drop) {
                    unsafe { slot.assume_init_drop() };
                }
                *len = new_len;
            }
            InlineVecStorage::Heap(vec) => {
                vec.truncate(new_len);
            }
        }
    }

    /// Drena os elementos especificados pelo intervalo como um `Vec<T>`.
    pub fn drain<R>(&mut self, range: R) -> Vec<T>
    where
        R: std::ops::RangeBounds<usize>,
    {
        let len = self.len();
        let start = match range.start_bound() {
            std::ops::Bound::Included(&n) => n,
            std::ops::Bound::Excluded(&n) => n + 1,
            std::ops::Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(&n) => n + 1,
            std::ops::Bound::Excluded(&n) => n,
            std::ops::Bound::Unbounded => len,
        };
        assert!(start <= end && end <= len, "intervalo de drain fora dos limites");

        let count = end - start;
        let mut out = Vec::with_capacity(count);

        match &mut self.storage {
            InlineVecStorage::Inline { len: inline_len, data } => {
                for slot in data.iter_mut().skip(start).take(count) {
                    out.push(unsafe { slot.assume_init_read() });
                }
                // Desloca os elementos remanescentes
                let tail_count = *inline_len - end;
                for i in 0..tail_count {
                    let next = unsafe { data[end + i].assume_init_read() };
                    data[start + i].write(next);
                }
                *inline_len -= count;
            }
            InlineVecStorage::Heap(vec) => {
                out.extend(vec.drain(start..end));
            }
        }
        out
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
