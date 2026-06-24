use super::*;
use crate::utils::hex::encode as hex_encode;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Compression method for cached resources


/// Compress data using specified method
pub fn compress_data(
    data: &[u8],
    method: CompressionMethod,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match method {
        CompressionMethod::None => Ok(data.to_vec()),
        CompressionMethod::Gzip => {
            let mut encoder =
                flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            use std::io::Write;
            encoder.write_all(data)?;
            Ok(encoder.finish()?)
        }
        CompressionMethod::Brotli => {
            let mut writer = brotli::CompressorWriter::new(Vec::new(), 4096, 11, 22);
            use std::io::Write;
            writer.write_all(data)?;
            Ok(writer.into_inner())
        }
        CompressionMethod::Zstd => Ok(zstd::encode_all(data, 3)?),
    }
}
