//! # Coleções de Alta Performance
//!
//! Estruturas de dados especializadas para renderização web sem overhead de alocação.

pub mod ancestor_filter;
pub mod bitset;
pub mod bloom_filter;
pub mod chunk_buffer;
pub mod flat_map;
pub mod inline_vec;
pub mod intrusive_list;
pub mod lru_cache;
pub mod ring_buffer;
pub mod triple_buffer;
pub mod utils;

pub use ancestor_filter::AncestorFilter;
pub use bitset::{AtomicBitSet, FixedBitSet};
pub use bloom_filter::BloomFilter;
pub use chunk_buffer::ChunkBuffer;
pub use flat_map::{FlatMap, FlatSet};
pub use inline_vec::InlineVec;
pub use intrusive_list::{IntrusiveLink, IntrusiveList, IntrusiveNode};
pub use lru_cache::LruCache;
pub use ring_buffer::RingBuffer;
pub use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet, FxHasher};
pub use triple_buffer::{triple_buffer, TripleBufferConsumer, TripleBufferProducer};
pub use utils::{chunk_slice, has_single_bit, next_power_of_two, popcount_u64};
