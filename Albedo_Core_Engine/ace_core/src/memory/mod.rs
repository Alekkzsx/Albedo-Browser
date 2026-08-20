//! # Gestão de Memória e Pressão do Sistema
//!
//! Barramentos de evento de pressão de memória (`MemoryPressureListener`), ponteiros fracos anti-ciclo (`WeakPtr`) e telemetria de RAM.

pub mod pressure;
pub mod tracing;
pub mod weak_ptr;

pub use pressure::{MemoryPressureLevel, MemoryPressureListener};
pub use tracing::{GCRoot, RootSet, Traceable, Visitor};
pub use weak_ptr::{LocalWeakPtr, LocalWeakPtrFactory, WeakPtr, WeakPtrFactory};
