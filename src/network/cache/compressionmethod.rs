use super::*;
use crate::utils::hex::encode as hex_encode;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Compression method for cached resources

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMethod {
    None,
    Gzip,
    Brotli,
    Zstd,
}

impl CompressionMethod {
    /// TODO: add docs
    pub fn as_str(&self) -> &'static str {
        match self {
            CompressionMethod::None => "none",
            CompressionMethod::Gzip => "gzip",
            CompressionMethod::Brotli => "brotli",
            CompressionMethod::Zstd => "zstd",
        }
    }

    /// TODO: add docs
    pub fn from_str(s: &str) -> Self {
        match s {
            "gzip" => CompressionMethod::Gzip,
            "brotli" => CompressionMethod::Brotli,
            "zstd" => CompressionMethod::Zstd,
            _ => CompressionMethod::None,
        }
    }
}
