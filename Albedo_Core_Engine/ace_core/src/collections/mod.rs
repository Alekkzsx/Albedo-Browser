//! # Coleções de Alta Performance
//!
//! Estruturas de dados especializadas para renderização web sem overhead de alocação.

pub mod bitset;
pub mod bloom_filter;
pub mod inline_vec;
pub mod utils;

pub use bitset::{FixedBitSet, AtomicBitSet};
pub use bloom_filter::BloomFilter;
pub use inline_vec::InlineVec;
pub use utils::{next_power_of_two, popcount_u64, has_single_bit, chunk_slice};
pub use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet, FxHasher};
