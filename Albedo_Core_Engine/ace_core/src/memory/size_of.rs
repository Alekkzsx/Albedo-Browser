//! # Telemetria Recursiva de Memória Heap (Servo MallocSizeOf & Gecko nsIMemoryReporter Pattern)
//!
//! Permite que qualquer estrutura de dados ou subsistema do motor meça com exatidão
//! a quantidade de bytes que retém no heap, viabilizando diagnósticos de vazamento de memória
//! e limpeza cirúrgica de caches sob eventos de `MemoryPressureListener`.

use std::mem::size_of;

/// Contrato para medição da quantidade de bytes de memória heap retidos por um objeto.
pub trait MallocSizeOf {
    /// Retorna o tamanho em bytes alocados dinamicamente no heap para esta instância.
    fn size_of_heap(&self) -> usize;
}

// Implementações para primitivos (0 bytes no heap)
macro_rules! impl_malloc_size_of_primitive {
    ($($t:ty),*) => {
        $(
            impl MallocSizeOf for $t {
                #[inline]
                fn size_of_heap(&self) -> usize {
                    0
                }
            }
        )*
    };
}

impl_malloc_size_of_primitive!(
    bool, char, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64
);

impl<T: MallocSizeOf> MallocSizeOf for Option<T> {
    #[inline]
    fn size_of_heap(&self) -> usize {
        match self {
            Some(v) => v.size_of_heap(),
            None => 0,
        }
    }
}

impl<T: MallocSizeOf> MallocSizeOf for Box<T> {
    #[inline]
    fn size_of_heap(&self) -> usize {
        size_of::<T>() + self.as_ref().size_of_heap()
    }
}

impl<T: MallocSizeOf> MallocSizeOf for Vec<T> {
    fn size_of_heap(&self) -> usize {
        let shallow = self.capacity() * size_of::<T>();
        let deep: usize = self.iter().map(|item| item.size_of_heap()).sum();
        shallow + deep
    }
}

impl MallocSizeOf for String {
    #[inline]
    fn size_of_heap(&self) -> usize {
        self.capacity()
    }
}

impl MallocSizeOf for smol_str::SmolStr {
    #[inline]
    fn size_of_heap(&self) -> usize {
        if self.is_heap_allocated() {
            self.len()
        } else {
            0
        }
    }
}

impl<T: MallocSizeOf, const N: usize> MallocSizeOf for crate::collections::InlineVec<T, N> {
    fn size_of_heap(&self) -> usize {
        let shallow = if self.is_heap() {
            self.capacity() * size_of::<T>()
        } else {
            0
        };
        let deep: usize = self.iter().map(|item| item.size_of_heap()).sum();
        shallow + deep
    }
}

impl<T: MallocSizeOf> MallocSizeOf for crate::collections::ThinVec<T> {
    fn size_of_heap(&self) -> usize {
        if self.capacity() > 0 {
            self.capacity() * size_of::<T>() + size_of::<usize>() * 2
        } else {
            0
        }
    }
}

impl<K: MallocSizeOf, V: MallocSizeOf> MallocSizeOf for crate::collections::FlatMap<K, V> {
    fn size_of_heap(&self) -> usize {
        let shallow = self.capacity() * (size_of::<K>() + size_of::<V>());
        let deep: usize = self
            .iter()
            .map(|(k, v)| k.size_of_heap() + v.size_of_heap())
            .sum();
        shallow + deep
    }
}

impl<T: MallocSizeOf> MallocSizeOf for crate::collections::FlatSet<T> {
    fn size_of_heap(&self) -> usize {
        let shallow = self.len() * size_of::<T>();
        let deep: usize = self.iter().map(|x| x.size_of_heap()).sum();
        shallow + deep
    }
}

impl<const WORDS: usize> MallocSizeOf for crate::collections::FixedBitSet<WORDS> {
    #[inline]
    fn size_of_heap(&self) -> usize {
        0 // Alocado 100% inline na struct
    }
}

impl<const WORDS: usize> MallocSizeOf for crate::collections::AtomicBitSet<WORDS> {
    #[inline]
    fn size_of_heap(&self) -> usize {
        0 // Alocado 100% inline na struct
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_malloc_size_of_vec_and_inline() {
        let mut v: Vec<u8> = Vec::with_capacity(1024);
        v.push(1);
        assert_eq!(v.size_of_heap(), 1024);

        let mut s = crate::collections::InlineVec::<u8, 16>::new();
        s.push(1);
        assert_eq!(s.size_of_heap(), 0); // Está na stack

        // Força transbordamento para o heap
        for i in 0..32 {
            s.push(i);
        }
        assert!(s.size_of_heap() >= 32);
    }
}
