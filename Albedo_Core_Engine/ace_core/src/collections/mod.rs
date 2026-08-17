//! # Coleções de Alta Performance
//!
//! Estruturas de dados especializadas para renderização web sem overhead de alocação.

pub mod bitset;
pub mod bloom_filter;
pub mod chunk_buffer;
pub mod inline_vec;
pub mod ring_buffer;
pub mod utils;

pub use bitset::{AtomicBitSet, FixedBitSet};
pub use bloom_filter::BloomFilter;
pub use chunk_buffer::ChunkBuffer;
pub use inline_vec::InlineVec;
pub use ring_buffer::RingBuffer;
pub use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet, FxHasher};
pub use utils::{chunk_slice, has_single_bit, next_power_of_two, popcount_u64};
