// ============================================================================
// Albedo Core Engine (ACE)
// File: sync.rs
// Description: Sincronização de Threads Ultra-Baixa Latência (SpinLock).
//              Ideal para filas de trabalho críticas de Microsegundos (ex: JIT).
// Author: Albedo Browser Engineering Team
// ============================================================================

//! # SpinLocks Nativo
//!
//! Um `Mutex` bloqueia a Thread chamando o Kernel do SO, o que gasta milhares de
//! ciclos de CPU de Overhead. O `SpinLock` gira num loop infinito usando
//! Compare-and-Swap (CAS), poupando chamadas de SO para trechos extremamente
//! curtos e críticos de dados.

use core::hint::spin_loop;
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

/// Bloqueio Ativo (Spin) focado em performance pura e bruta.
/// *Atenção:* Nunca utilize para aguardar I/O, apenas para gerência de memória Rápida!
pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for SpinLock<T> {}
unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }

    /// Adquire a posse dos dados girando no processador ativamente (com Backoff Adaptativo).
    #[inline]
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        let mut backoff = 1;
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Se falhou (alguém tem o Lock), gira com exponential backoff
            while self.locked.load(Ordering::Relaxed) {
                for _ in 0..backoff {
                    spin_loop();
                }
                // Cap no backoff para não causar latência imensa no destravamento (Max 64 ciclos)
                if backoff < 64 {
                    backoff *= 2;
                }
            }
        }

        SpinLockGuard { lock: self }
    }
}

/// Garantidor de Liberação do Lock (RAII). Segura a referência mutável segura.
pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        // Libera o lock atômico imediatamente
        self.lock.locked.store(false, Ordering::Release);
    }
}
