//! # Gestão de Memória e Pressão do Sistema
//!
//! Barramentos de evento de pressão de memória (`MemoryPressureListener`) e telemetria de RAM.

pub mod pressure;
pub mod tracing;

pub use pressure::{MemoryPressureLevel, MemoryPressureListener};
pub use tracing::{GCRoot, RootSet, Traceable, Visitor};
