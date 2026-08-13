// ============================================================================
// Albedo Core Engine (ACE)
// File: arena.rs
// Description: Bump-Pointer Arena Allocator para alocação massiva em O(1).
//              Ideal para ciclos de vida acoplados (ex: DOM Tree).
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::cell::Cell;
use std::mem;
use std::ptr::{self, NonNull};
use std::alloc::{alloc, dealloc, Layout};

const CHUNK_SIZE: usize = 64 * 1024; // Páginas de 64KB

struct Chunk {
    ptr: NonNull<u8>,
    capacity: usize,
    /// Offset atual em bytes a partir do início (Bump Pointer)
    len: Cell<usize>,
    next: Cell<Option<NonNull<Chunk>>>,
}

impl Chunk {
    fn new(size: usize) -> Self {
        let layout = Layout::from_size_align(size, mem::align_of::<u64>()).unwrap();
        let ptr = unsafe { alloc(layout) };
        assert!(!ptr.is_null(), "OOM allocating Arena chunk");
        
        Self {
            ptr: NonNull::new(ptr).unwrap(),
            capacity: size,
            len: Cell::new(0),
            next: Cell::new(None),
        }
    }

    fn drop_chunk(&mut self) {
        let layout = Layout::from_size_align(self.capacity, mem::align_of::<u64>()).unwrap();
        unsafe { dealloc(self.ptr.as_ptr(), layout) };
    }
}

/// Alocador Linear (Bump-Pointer).
/// 
/// Solicita blocos colossais e destrói o conceito de `free` individual.
/// Toda a memória é liberada O(1) quando a Arena sai de escopo.
pub struct Arena {
    head: Cell<NonNull<Chunk>>,
}

impl Arena {
    pub fn new() -> Self {
        let chunk = Box::new(Chunk::new(CHUNK_SIZE));
        Self {
            head: Cell::new(NonNull::from(Box::leak(chunk))),
        }
    }

    /// Aloca uma estrutura T e retorna uma referência mutável.
    /// A alocação é puramente a soma de um ponteiro e alinhamento matemático.
    #[inline]
    pub fn alloc<T>(&self, value: T) -> &mut T {
        unsafe {
            let layout = Layout::new::<T>();
            let size = layout.size();
            let align = layout.align();
            
            if size == 0 {
                // Para Zero-Sized Types, apenas retornamos um ponteiro dangling
                let ptr = NonNull::<T>::dangling().as_ptr();
                ptr::write(ptr, value);
                return &mut *ptr;
            }

            let mut current_chunk = self.head.get().as_ref();
            let mut start = current_chunk.ptr.as_ptr() as usize + current_chunk.len.get();
            let mut padding = start.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
            padding = padding.wrapping_sub(start);
            
            let mut needed = size + padding;

            // Se o chunk não suportar a alocação, cria um novo
            if current_chunk.len.get() + needed > current_chunk.capacity {
                let new_cap = std::cmp::max(CHUNK_SIZE, needed);
                let new_chunk = Box::new(Chunk::new(new_cap));
                let new_chunk_ptr = NonNull::from(Box::leak(new_chunk));
                
                // Encadeia e torna o novo chunk como 'head'
                new_chunk_ptr.as_ref().next.set(Some(self.head.get()));
                self.head.set(new_chunk_ptr);
                
                current_chunk = self.head.get().as_ref();
                start = current_chunk.ptr.as_ptr() as usize;
                padding = start.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
                padding = padding.wrapping_sub(start);
                needed = size + padding;
            }

            // O Bump em si: O(1) puro
            let offset = current_chunk.len.get();
            let ptr_val = current_chunk.ptr.as_ptr().add(offset + padding) as *mut T;
            
            // Grava o bit na memória
            ptr::write(ptr_val, value);
            current_chunk.len.set(offset + needed);

            &mut *ptr_val
        }
    }

    /// Limpa a Arena para reaproveitamento (Phase-Oriented Allocation).
    /// O chunk atual (que concentra a capacidade do último ciclo) é mantido
    /// e resetado O(1), enquanto chunks antigos são liberados.
    pub fn clear(&self) {
        unsafe {
            let head_ptr = self.head.get();
            let head = head_ptr.as_ref();
            
            // O(1) reset do chunk atual
            head.len.set(0);
            
            // Libera chunks antigos que sobraram na cauda
            let mut current = head.next.get();
            head.next.set(None);
            
            while let Some(mut chunk_ptr) = current {
                let chunk = chunk_ptr.as_mut();
                current = chunk.next.get();
                chunk.drop_chunk();
                let _ = Box::from_raw(chunk_ptr.as_ptr());
            }
        }
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        unsafe {
            let mut current = Some(self.head.get());
            while let Some(mut chunk_ptr) = current {
                let chunk = chunk_ptr.as_mut();
                current = chunk.next.get();
                chunk.drop_chunk();
                // Liberar o Box estrutural do Chunk
                let _ = Box::from_raw(chunk_ptr.as_ptr());
            }
        }
    }
}


