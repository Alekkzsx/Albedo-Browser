//! # Coleções de Alta Performance
//!
//! Estruturas de dados especializadas para renderização web sem overhead de alocação.

pub mod bitset;
pub mod utils;

pub use bitset::{FixedBitSet, AtomicBitSet};
pub use utils::{next_power_of_two, popcount_u64, has_single_bit, chunk_slice};
