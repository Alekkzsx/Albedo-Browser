use super::*;
use crate::utils::hex::encode as hex_encode;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Compression method for cached resources


/// Disk cache manager for HTTP resources
pub struct DiskCache {
    pub content_dir: PathBuf,
    pub conn: std::sync::Arc<std::sync::Mutex<Connection>>,
    pub max_size_bytes: u64, // 500 MB default
}
