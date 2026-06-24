use super::*;
use crate::utils::hex::encode as hex_encode;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Compression method for cached resources


/// Disk cache manager for HTTP resources
pub struct DiskCache {
    content_dir: PathBuf,
    conn: std::sync::Arc<std::sync::Mutex<Connection>>,
    max_size_bytes: u64, // 500 MB default
}
