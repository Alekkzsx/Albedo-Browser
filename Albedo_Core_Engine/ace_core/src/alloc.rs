// ============================================================================
// Albedo Core Engine (ACE)
// File: alloc.rs
// Description: Custom Global Allocator para rastrear e perfilhar o Heap,
//              prevenindo vazamentos (Memory Leaks) nativamente.
// Author: Albedo Browser Engineering Team
// ============================================================================

//! # Interceptador de Alocação (Memory Profiler)
//! 
//! Para evitar o consumo caótico de memória, comum em navegadores modernos,
//! o `ace_core` intercepta cada `malloc` e `free`. Todas as estruturas
//! geradas pelo Rust (como Vectors e Strings) são pesadas.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Contadores atômicos globais para tracking massivo sem overhead de Locks.
pub static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
pub static ACTIVE_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

/// O Alocador Customizado do Albedo.
/// Intercepta as chamadas para registrar a contabilidade e delega a memória 
/// bruta ao alocador do sistema (`System`).
pub struct AlbedoAllocator;

unsafe impl GlobalAlloc for AlbedoAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Incrementa as métricas
        ALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        ACTIVE_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        
        // Delega de fato ao OS
        System.alloc(layout)
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Decrementa as métricas
        ALLOCATED_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        ACTIVE_ALLOCATIONS.fetch_sub(1, Ordering::Relaxed);
        
        // Libera no OS
        System.dealloc(ptr, layout)
    }
}

/// A Instância Global. Ao compilar com a crate ace_core, 
/// toda a memória passa por aqui.
#[global_allocator]
static GLOBAL_ALLOCATOR: AlbedoAllocator = AlbedoAllocator;

// ----------------------------------------------------------------------------
// Utilities
// ----------------------------------------------------------------------------

/// Retorna a quantidade de bytes atualmente consumidos pelo processo (Heap visível).
#[inline]
pub fn current_memory_usage_bytes() -> usize {
    ALLOCATED_BYTES.load(Ordering::Relaxed)
}

/// Retorna a quantidade de blocos de memória não-liberados (Vazamentos se > 0 no fim).
#[inline]
pub fn current_active_allocations() -> usize {
    ACTIVE_ALLOCATIONS.load(Ordering::Relaxed)
}
