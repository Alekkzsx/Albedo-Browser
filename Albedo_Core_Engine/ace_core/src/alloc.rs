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
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Contadores atômicos globais para tracking massivo sem overhead de Locks.
pub static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
pub static ACTIVE_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
pub static PEAK_MEMORY_BYTES: AtomicUsize = AtomicUsize::new(0);

const TLAC_BATCH_SIZE: usize = 64 * 1024; // 64 KB

thread_local! {
    // Usamos `const` block para garantir inicialização Zero-Cost (sem chamadas a `malloc`).
    // Isso evita Stack Overflow recursivo (onde inicializar o thread_local chama malloc,
    // que chama o thread_local, infinitamente).
    static LOCAL_ALLOC_BYTES: Cell<usize> = const { Cell::new(0) };
    static LOCAL_DEALLOC_BYTES: Cell<usize> = const { Cell::new(0) };
    static REENTRANCY_GUARD: Cell<bool> = const { Cell::new(false) };
}

/// O Alocador Customizado do Albedo com TLAC.
pub struct AlbedoAllocator;

unsafe impl GlobalAlloc for AlbedoAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ACTIVE_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        
        // Tenta usar o cache local
        let size = layout.size();
        let mut bypass_tlac = true;
        
        REENTRANCY_GUARD.with(|guard| {
            if !guard.get() {
                guard.set(true);
                bypass_tlac = false;
                
                LOCAL_ALLOC_BYTES.with(|local| {
                    let mut current = local.get();
                    current += size;
                    
                    if current >= TLAC_BATCH_SIZE {
                        // Flush batch to global
                        let total = ALLOCATED_BYTES.fetch_add(current, Ordering::Relaxed) + current;
                        PEAK_MEMORY_BYTES.fetch_max(total, Ordering::Relaxed);
                        local.set(0);
                    } else {
                        local.set(current);
                    }
                });
                
                guard.set(false);
            }
        });

        if bypass_tlac {
            // Fallback direto no global se houver reentrância (ex: inicialização do thread_local)
            let total = ALLOCATED_BYTES.fetch_add(size, Ordering::Relaxed) + size;
            PEAK_MEMORY_BYTES.fetch_max(total, Ordering::Relaxed);
        }

        // Delega de fato ao OS
        System.alloc(layout)
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        ACTIVE_ALLOCATIONS.fetch_sub(1, Ordering::Relaxed);
        
        let size = layout.size();
        let mut bypass_tlac = true;
        
        REENTRANCY_GUARD.with(|guard| {
            if !guard.get() {
                guard.set(true);
                bypass_tlac = false;
                
                LOCAL_DEALLOC_BYTES.with(|local| {
                    let mut current = local.get();
                    current += size;
                    
                    if current >= TLAC_BATCH_SIZE {
                        // Flush batch to global
                        ALLOCATED_BYTES.fetch_sub(current, Ordering::Relaxed);
                        local.set(0);
                    } else {
                        local.set(current);
                    }
                });
                
                guard.set(false);
            }
        });

        if bypass_tlac {
            ALLOCATED_BYTES.fetch_sub(size, Ordering::Relaxed);
        }

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

/// Retorna o pico máximo histórico de bytes consumidos (High-water mark).
#[inline]
pub fn peak_memory_usage_bytes() -> usize {
    PEAK_MEMORY_BYTES.load(Ordering::Relaxed)
}
