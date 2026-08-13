// ============================================================================
// Albedo Core Engine (ACE)
// File: small_vec.rs
// Description: Estrutura SmallVec otimizada para manter N elementos na Stack
//              e só alocar no Heap se exceder a capacidade, essencial para o DOM.
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::ptr;
use std::slice;

pub enum SmallVecStorage<T, const N: usize> {
    Inline {
        buffer: [MaybeUninit<T>; N],
        len: usize,
    },
    Heap(Vec<T>),
}

/// Vetor Otimizado para pequenos tamanhos.
/// Armazena até `N` elementos na pilha, e usa fallback para `Vec<T>` no Heap.
pub struct SmallVec<T, const N: usize> {
    storage: SmallVecStorage<T, N>,
}

impl<T, const N: usize> SmallVec<T, N> {
    #[inline]
    pub fn new() -> Self {
        Self {
            storage: SmallVecStorage::Inline {
                // SAFETY: MaybeUninit is safe to assume uninitialized
                buffer: unsafe { MaybeUninit::uninit().assume_init() },
                len: 0,
            },
        }
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        match &mut self.storage {
            SmallVecStorage::Inline { buffer, len } => {
                if *len < N {
                    // Tem espaço inline
                    buffer[*len].write(value);
                    *len += 1;
                } else {
                    // Sem espaço inline: Promove para o Heap
                    let mut vec = Vec::with_capacity(N + 1);
                    // SAFETY: O tamanho do array Inline nunca excede N. `copy_nonoverlapping` move bits
                    // em memória, o que é seguro pois Inline não será mais usado, mitigando double-drop.
                    unsafe {
                        // Move os elementos da stack para o heap sem disparar destructors
                        let inline_ptr = buffer.as_ptr() as *const T;
                        ptr::copy_nonoverlapping(inline_ptr, vec.as_mut_ptr(), N);
                        vec.set_len(N);

                        // O buffer da stack foi esvaziado logicamente.
                        // Agora escrevemos o novo valor.
                        vec.push(value);
                    }
                    self.storage = SmallVecStorage::Heap(vec);
                }
            }
            SmallVecStorage::Heap(vec) => {
                vec.push(value);
            }
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match &self.storage {
            SmallVecStorage::Inline { len, .. } => *len,
            SmallVecStorage::Heap(vec) => vec.len(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T, const N: usize> Default for SmallVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

// Para usar o SmallVec como um slice normal
impl<T, const N: usize> Deref for SmallVec<T, N> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match &self.storage {
            // SAFETY: A slice criada mapeia com precisão a área efetivamente preenchida na stack (indicada por len).
            SmallVecStorage::Inline { buffer, len } => unsafe {
                slice::from_raw_parts(buffer.as_ptr() as *const T, *len)
            },
            SmallVecStorage::Heap(vec) => vec.as_slice(),
        }
    }
}

impl<T, const N: usize> DerefMut for SmallVec<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.storage {
            // SAFETY: A slice criada mapeia com precisão a área mutável efetivamente preenchida na stack.
            SmallVecStorage::Inline { buffer, len } => unsafe {
                slice::from_raw_parts_mut(buffer.as_mut_ptr() as *mut T, *len)
            },
            SmallVecStorage::Heap(vec) => vec.as_mut_slice(),
        }
    }
}

impl<T, const N: usize> Drop for SmallVec<T, N> {
    fn drop(&mut self) {
        if let SmallVecStorage::Inline { buffer, len } = &mut self.storage {
            // SAFETY: Chamamos drop explicitamente nos elementos empilhados, o que é mandatório pois
            // arranjos de MaybeUninit não disparam drop por padrão, o que causaria Memory Leak.
            unsafe {
                // Temos que rodar os drops manuais, senão teremos vazamentos de memória (ex: SmallVec<String, 4>)
                let slice = slice::from_raw_parts_mut(buffer.as_mut_ptr() as *mut T, *len);
                ptr::drop_in_place(slice);
            }
        }
        // Se for Heap(Vec), o Drop do próprio Vec vai resolver o resto
    }
}
