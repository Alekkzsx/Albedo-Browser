//! # Coleções de Alta Performance
//!
//! Estruturas de dados especializadas para renderização web sem overhead de alocação.

pub mod bitset;

pub use bitset::{FixedBitSet, AtomicBitSet};
