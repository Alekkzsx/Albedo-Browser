//! # Subsistema de Cache HTTP (RFC 9111)
//!
//! Fornece armazenamento LRU particionado com cálculo de frescor normativo,
//! revalidação condicional via 304 e isolamento de estado contra rastreamento cross-site.

pub mod entry;
pub mod partition;
pub mod storage;

pub use entry::CacheEntry;
pub use partition::NetworkIsolationKey;
pub use storage::{CacheStats, HttpCache};
