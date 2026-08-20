//! # Vetor Compacto de Palavra Única (Servo ThinVec Pattern)
//!
//! Em nós da árvore DOM e caixas de layout onde existem milhões de instâncias em memória,
//! o `Vec<T>` padrão do Rust ocupa **24 bytes** (3 palavras: ponteiro, capacidade, tamanho) mesmo quando vazio.
//!
//! O `ThinVec<T>` armazena os metadados de capacidade e tamanho no cabeçalho da alocação no heap,
//! ocupando **exatos 8 bytes (1 palavra / 64 bits)** no corpo da struct (graças ao Niche Optimization de `Option<NonNull>`),
//! economizando até 60% de metadados em árvores densas.

use std::alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout};
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

#[repr(C)]
struct ThinHeader {
    capacity: usize,
    len: usize,
}

/// Vetor compacto de tamanho de struct de 8 bytes (1 ponteiro).
pub struct ThinVec<T> {
    ptr: Option<NonNull<ThinHeader>>,
    _marker: std::marker::PhantomData<T>,
}

unsafe impl<T: Send> Send for ThinVec<T> {}
unsafe impl<T: Sync> Sync for ThinVec<T> {}

impl<T> Default for ThinVec<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ThinVec<T> {
    /// Cria um novo `ThinVec` vazio ocupando 8 bytes e 0 alocações no heap.
    #[inline]
    pub const fn new() -> Self {
        Self {
            ptr: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// Cria um `ThinVec` com capacidade pré-alocada.
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity == 0 {
            Self::new()
        } else {
            let mut vec = Self::new();
            vec.grow(capacity);
            vec
        }
    }

    /// Retorna o layout de memória para um cabeçalho com a capacidade especificada.
    fn layout_for(capacity: usize) -> (Layout, usize) {
        let header_layout = Layout::new::<ThinHeader>();
        let (data_layout, data_offset) = Layout::array::<T>(capacity)
            .map(|l| header_layout.extend(l).unwrap())
            .unwrap_or_else(|_| panic!("Overflow de tamanho de capacidade no ThinVec"));
        (data_layout.pad_to_align(), data_offset)
    }

    #[inline]
    fn header(&self) -> Option<&ThinHeader> {
        self.ptr.map(|p| unsafe { p.as_ref() })
    }

    #[inline]
    fn header_mut(&mut self) -> Option<&mut ThinHeader> {
        self.ptr.map(|mut p| unsafe { p.as_mut() })
    }

    #[inline]
    fn data_ptr(&self) -> *mut T {
        match self.ptr {
            Some(p) => {
                let (_, data_offset) = Self::layout_for(0);
                unsafe { (p.as_ptr() as *mut u8).add(data_offset) as *mut T }
            }
            None => NonNull::<T>::dangling().as_ptr(),
        }
    }

    /// Retorna a quantidade de elementos armazenados.
    #[inline]
    pub fn len(&self) -> usize {
        self.header().map_or(0, |h| h.len)
    }

    /// Retorna `true` se o vetor estiver vazio.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Retorna a capacidade atual do buffer no heap.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.header().map_or(0, |h| h.capacity)
    }

    /// Expande a capacidade do vetor.
    fn grow(&mut self, min_cap: usize) {
        let current_cap = self.capacity();
        let new_cap = (current_cap * 2).max(min_cap).max(4);

        let (new_layout, _) = Self::layout_for(new_cap);

        let new_ptr = match self.ptr {
            Some(old_ptr) => {
                let (old_layout, _) = Self::layout_for(current_cap);
                let raw = unsafe {
                    realloc(old_ptr.as_ptr() as *mut u8, old_layout, new_layout.size())
                };
                if raw.is_null() {
                    handle_alloc_error(new_layout);
                }
                NonNull::new(raw as *mut ThinHeader).unwrap()
            }
            None => {
                let raw = unsafe { alloc(new_layout) };
                if raw.is_null() {
                    handle_alloc_error(new_layout);
                }
                let ptr = NonNull::new(raw as *mut ThinHeader).unwrap();
                unsafe {
                    std::ptr::write(
                        ptr.as_ptr(),
                        ThinHeader {
                            capacity: new_cap,
                            len: 0,
                        },
                    );
                }
                ptr
            }
        };

        unsafe {
            (*new_ptr.as_ptr()).capacity = new_cap;
        }
        self.ptr = Some(new_ptr);
    }

    /// Adiciona um novo elemento ao final do vetor.
    pub fn push(&mut self, value: T) {
        let len = self.len();
        if len == self.capacity() {
            self.grow(len + 1);
        }

        unsafe {
            let elem_ptr = self.data_ptr().add(len);
            std::ptr::write(elem_ptr, value);
            self.header_mut().unwrap().len += 1;
        }
    }

    /// Remove e retorna o último elemento do vetor, se existir.
    pub fn pop(&mut self) -> Option<T> {
        let len = self.len();
        if len == 0 {
            None
        } else {
            unsafe {
                let new_len = len - 1;
                self.header_mut().unwrap().len = new_len;
                let elem_ptr = self.data_ptr().add(new_len);
                Some(std::ptr::read(elem_ptr))
            }
        }
    }

    /// Remove o elemento no índice especificado, deslocando os elementos subsequentes.
    pub fn remove(&mut self, index: usize) -> T {
        let len = self.len();
        assert!(index < len, "Índice fora dos limites no ThinVec::remove");

        unsafe {
            let elem_ptr = self.data_ptr().add(index);
            let val = std::ptr::read(elem_ptr);
            let count = len - index - 1;
            if count > 0 {
                std::ptr::copy(elem_ptr.add(1), elem_ptr, count);
            }
            self.header_mut().unwrap().len -= 1;
            val
        }
    }

    /// Limpa todos os elementos do vetor chamando seus destrutores.
    pub fn clear(&mut self) {
        let len = self.len();
        if len > 0 {
            unsafe {
                let data = self.data_ptr();
                self.header_mut().unwrap().len = 0;
                let slice = std::slice::from_raw_parts_mut(data, len);
                std::ptr::drop_in_place(slice);
            }
        }
    }

    /// Retorna uma fatia imutável dos elementos.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        let len = self.len();
        if len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.data_ptr(), len) }
        }
    }

    /// Retorna uma fatia mutável dos elementos.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let len = self.len();
        if len == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(self.data_ptr(), len) }
        }
    }
}

impl<T> Deref for ThinVec<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> DerefMut for ThinVec<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T> Drop for ThinVec<T> {
    fn drop(&mut self) {
        if let Some(ptr) = self.ptr {
            self.clear();
            let capacity = unsafe { ptr.as_ref().capacity };
            let (layout, _) = Self::layout_for(capacity);
            unsafe {
                dealloc(ptr.as_ptr() as *mut u8, layout);
            }
        }
    }
}

impl<T: Clone> Clone for ThinVec<T> {
    fn clone(&self) -> Self {
        let mut new_vec = Self::with_capacity(self.len());
        for item in self.as_slice() {
            new_vec.push(item.clone());
        }
        new_vec
    }
}

impl<T: fmt::Debug> fmt::Debug for ThinVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}

impl<T: PartialEq> PartialEq for ThinVec<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Eq> Eq for ThinVec<T> {}

impl<T> FromIterator<T> for ThinVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (lower, _) = iter.size_hint();
        let mut vec = Self::with_capacity(lower);
        for item in iter {
            vec.push(item);
        }
        vec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thin_vec_size_in_struct() {
        // O ThinVec deve ocupar estritamente 8 bytes na memória (1 ponteiro de 64-bit)
        assert_eq!(std::mem::size_of::<ThinVec<u64>>(), 8);
        assert_eq!(std::mem::size_of::<ThinVec<String>>(), 8);
    }

    #[test]
    fn test_thin_vec_push_pop_and_slice() {
        let mut vec = ThinVec::new();
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 0);

        vec.push(10);
        vec.push(20);
        vec.push(30);

        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], 10);
        assert_eq!(vec[1], 20);
        assert_eq!(vec[2], 30);
        assert_eq!(vec.as_slice(), &[10, 20, 30]);

        assert_eq!(vec.pop(), Some(30));
        assert_eq!(vec.len(), 2);
        assert_eq!(vec.remove(0), 10);
        assert_eq!(vec.as_slice(), &[20]);
    }
}
