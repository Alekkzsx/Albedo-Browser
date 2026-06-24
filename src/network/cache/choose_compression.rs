use super::*;
use crate::utils::hex::encode as hex_encode;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Compression method for cached resources


/// Choose best compression method based on content type
pub fn choose_compression(content_type: &str) -> CompressionMethod {
    match content_type {
        t if t.contains("text") || t.contains("javascript") || t.contains("json") => {
            CompressionMethod::Gzip
        }
        t if t.contains("image") => {
            // Images are usually already compressed
            CompressionMethod::None
        }
        _ => CompressionMethod::Gzip, // Default to gzip for everything else
    }
}
