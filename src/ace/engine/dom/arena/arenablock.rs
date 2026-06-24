use super::*;
//! DomArena - Arena de alocação para AceDOM nodes
//! 
//! Implementa alocação em blocos de 4KB para melhor performance
//! e memória cache-friendly.

use std::alloc::{self, Layout};
use std::cell::Cell;
use std::ptr::NonNull;

/// Tamanho do bloco da arena em bytes (4KB)


/// Um bloco de memória na arena
pub(crate) struct ArenaBlock {
    data: NonNull<[ArenaNode]>,
    capacity: usize,
    len: Cell<usize>,
}

impl ArenaBlock {
pub(crate) fn new(capacity: usize) -> Self {
        let layout = Layout::array::<ArenaNode>(capacity).expect("Albedo Engine: internal invariant violated");
        // SAFETY: We allocate memory for ArenaNode array and initialize each element.
        // The allocation is checked for null and handle_alloc_error is called on failure.
        unsafe {
            let ptr = alloc::alloc(layout);
            if ptr.is_null() {
                alloc::handle_alloc_error(layout);
            }
            
            // Inicializar memória
            for i in 0..capacity {
                std::ptr::write(ptr.add(i) as *mut ArenaNode, ArenaNode::new(
                    crate::ace::engine::dom::AceNodeType::Document
                ));
            }
            
            let slice = std::slice::from_raw_parts_mut(ptr as *mut ArenaNode, capacity);
            Self {
                data: NonNull::new_unchecked(slice),
                capacity,
                len: Cell::new(0),
            }
        }
    }
    
pub(crate) fn push(&self, node: ArenaNode) -> Option<usize> {
        let current_len = self.len.get();
        if current_len >= self.capacity {
            return None;
        }
        
        // SAFETY: current_len is within bounds (checked above).
        // The pointer is valid for the arena's lifetime.
        unsafe {
            let ptr = self.data.as_ptr() as *mut ArenaNode;
            std::ptr::write(ptr.add(current_len), node);
        }
        
        self.len.set(current_len + 1);
        Some(current_len)
    }
    
pub(crate) fn get(&self, index: usize) -> Option<&ArenaNode> {
        if index >= self.len.get() {
            return None;
        }
        // SAFETY: index is within bounds (checked above).
        // The arena guarantees memory validity for its lifetime.
        unsafe { self.data.as_ref().get(index) }
    }
    
pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut ArenaNode> {
        if index >= self.len.get() {
            return None;
        }
        // SAFETY: index is within bounds (checked above).
        // We have exclusive access via &mut self.
        unsafe { self.data.as_mut().get_mut(index) }
    }
}

impl Drop for ArenaBlock {
pub(crate) fn drop(&mut self) {
        // SAFETY: The pointer was allocated with this layout in ArenaBlock::new.
        // We have exclusive access via &mut self in Drop.
        unsafe {
            let layout = Layout::array::<ArenaNode>(self.capacity).expect("Albedo Engine: internal invariant violated");
            alloc::dealloc(self.data.as_ptr() as *mut u8, layout);
        }
    }
}
