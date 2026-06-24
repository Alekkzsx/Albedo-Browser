use crate::utils::hex::encode as hex_encode;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Compression method for cached resources

pub mod compressionmethod; pub use compressionmethod::*;
pub mod cacheentry; pub use cacheentry::*;
pub mod diskcache; pub use diskcache::*;
pub mod diskcache_impl_1; pub use diskcache_impl_1::*;
pub mod diskcache_impl_2; pub use diskcache_impl_2::*;
pub mod diskcache_impl_3; pub use diskcache_impl_3::*;
pub mod compress_data; pub use compress_data::*;
pub mod decompress_data; pub use decompress_data::*;
pub mod choose_compression; pub use choose_compression::*;
