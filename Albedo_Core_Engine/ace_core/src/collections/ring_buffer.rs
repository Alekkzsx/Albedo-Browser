//! # Buffer Circular em Stack (Chromium base::RingBuffer Pattern)
//!
//! Buffer circular de capacidade fixa pré-alocado (zero alocações no heap),
//! ideal para telemetria de taxas de quadros a 120 FPS e janelas deslizantes de métricas.

/// Buffer circular com capacidade fixa alocada em stack/inline.
#[derive(Debug, Clone)]
pub struct RingBuffer<T, const N: usize> {
    data: [Option<T>; N],
    head: usize, // Aponta para a próxima posição de escrita
    len: usize,
}

impl<T, const N: usize> RingBuffer<T, N> {
    /// Cria um novo buffer circular vazio.
    pub fn new() -> Self {
        Self {
            data: std::array::from_fn(|_| None),
            head: 0,
            len: 0,
        }
    }

    /// Insere um elemento no buffer.
    ///
    /// Se o buffer estiver cheio, sobrescreve o elemento mais antigo e o devolve em `Some(old)`.
    pub fn push(&mut self, item: T) -> Option<T> {
        if N == 0 {
            return Some(item);
        }

        let old = self.data[self.head].take();
        self.data[self.head] = Some(item);
        self.head = (self.head + 1) % N;

        if self.len < N {
            self.len += 1;
        }

        old
    }

    /// Retorna a quantidade de elementos atualmente armazenados.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Retorna `true` se o buffer estiver vazio.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Retorna `true` se o buffer estiver em sua capacidade máxima.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len == N
    }

    /// Capacidade máxima do buffer.
    #[inline]
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Acessa um elemento por índice ordinal relativo (0 = mais antigo, len-1 = mais recente).
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len || N == 0 {
            return None;
        }

        let start = if self.len < N {
            0
        } else {
            self.head
        };

        let actual_idx = (start + index) % N;
        self.data[actual_idx].as_ref()
    }

    /// Limpa todos os elementos do buffer.
    pub fn clear(&mut self) {
        for slot in self.data.iter_mut() {
            *slot = None;
        }
        self.head = 0;
        self.len = 0;
    }

    /// Itera sobre os elementos em ordem cronológica (do mais antigo ao mais recente).
    pub fn iter(&self) -> RingBufferIter<'_, T, N> {
        RingBufferIter {
            buffer: self,
            current_index: 0,
        }
    }
}

impl<T, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterador em ordem cronológica sobre os elementos de um `RingBuffer`.
pub struct RingBufferIter<'a, T, const N: usize> {
    buffer: &'a RingBuffer<T, N>,
    current_index: usize,
}

impl<'a, T, const N: usize> Iterator for RingBufferIter<'a, T, N> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index < self.buffer.len() {
            let item = self.buffer.get(self.current_index);
            self.current_index += 1;
            item
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.buffer.len() - self.current_index;
        (remaining, Some(remaining))
    }
}

impl<'a, T, const N: usize> ExactSizeIterator for RingBufferIter<'a, T, N> {}
