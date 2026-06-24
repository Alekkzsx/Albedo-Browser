use super::*;
use crate::utils::hex::encode as hex_encode;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Compression method for cached resources


/// Decompress data using specified method
pub fn decompress_data(
    data: &[u8],
    method: CompressionMethod,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match method {
        CompressionMethod::None => Ok(data.to_vec()),
        CompressionMethod::Gzip => {
            use std::io::Read;
            let mut decoder = flate2::read::GzDecoder::new(data);
            let mut result = Vec::new();
            decoder.read_to_end(&mut result)?;
            Ok(result)
        }
        CompressionMethod::Brotli => {
            let mut result = Vec::new();
            use std::io::Read;
            let mut reader = brotli::Decompressor::new(data, 4096);
            reader.read_to_end(&mut result)?;
            Ok(result)
        }
        CompressionMethod::Zstd => Ok(zstd::decode_all(data)?),
    }
}
