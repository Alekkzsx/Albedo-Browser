use super::*;
use crate::utils::hex::encode as hex_encode;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Compression method for cached resources


/// Cached resource metadata
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub url: String,
    pub etag: Option<String>,
    pub cache_control: Option<String>,
    pub last_modified: Option<String>,
    pub expires: Option<String>,
    pub content_type: String,
    pub status_code: u16,
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression: CompressionMethod,
    pub timestamp: SystemTime,
    pub last_accessed: SystemTime,
    pub access_count: u32,
}
