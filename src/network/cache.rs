use crate::ace::util::hex::encode as hex_encode;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
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
    pub fn as_str(&self) -> &'static str {
        match self {
            CompressionMethod::None => "none",
            CompressionMethod::Gzip => "gzip",
            CompressionMethod::Brotli => "brotli",
            CompressionMethod::Zstd => "zstd",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "gzip" => CompressionMethod::Gzip,
            "brotli" => CompressionMethod::Brotli,
            "zstd" => CompressionMethod::Zstd,
            _ => CompressionMethod::None,
        }
    }
}

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

/// Disk cache manager for HTTP resources
pub struct DiskCache {
    content_dir: PathBuf,
    conn: std::sync::Arc<std::sync::Mutex<Connection>>,
    max_size_bytes: u64, // 500 MB default
}

impl DiskCache {
    /// Initialize disk cache with database and directories
    pub fn new(max_size_bytes: u64) -> Result<Self, Box<dyn std::error::Error>> {
        // Determine cache directory: $HOME/.cache/albedo/http_cache/
        let cache_dir = if let Some(cache_home) = dirs::cache_dir() {
            cache_home.join("albedo").join("http_cache")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home)
                .join(".cache")
                .join("albedo")
                .join("http_cache")
        } else {
            PathBuf::from("./cache/http_cache")
        };

        let content_dir = cache_dir.join("content");
        let db_path = cache_dir.join("cache.db");

        // Create directories
        std::fs::create_dir_all(&content_dir)?;

        // Initialize SQLite database
        let conn = Connection::open(&db_path)?;

        // Create tables if they don't exist
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS cache_entries (
                id INTEGER PRIMARY KEY,
                url TEXT UNIQUE NOT NULL,
                etag TEXT,
                cache_control TEXT,
                last_modified TEXT,
                expires TEXT,
                content_type TEXT NOT NULL,
                status_code INTEGER NOT NULL,
                original_size INTEGER NOT NULL,
                compressed_size INTEGER NOT NULL,
                compression TEXT NOT NULL,
                file_hash TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                last_accessed INTEGER NOT NULL,
                access_count INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_url ON cache_entries(url);
            CREATE INDEX IF NOT EXISTS idx_timestamp ON cache_entries(timestamp);",
        )?;

        Ok(DiskCache {
            content_dir,
            conn: std::sync::Arc::new(std::sync::Mutex::new(conn)),
            max_size_bytes,
        })
    }

    /// Get cached resource by URL
    pub fn get(
        &self,
        url: &str,
    ) -> Result<Option<(CacheEntry, Vec<u8>)>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT url, etag, cache_control, last_modified, expires, content_type, 
                    status_code, original_size, compressed_size, compression, file_hash, 
                    timestamp, last_accessed, access_count 
             FROM cache_entries WHERE url = ?1",
        )?;

        let result = stmt.query_row([url], |row| {
            Ok((
                row.get::<_, String>(0)?,         // url
                row.get::<_, Option<String>>(1)?, // etag
                row.get::<_, Option<String>>(2)?, // cache_control
                row.get::<_, Option<String>>(3)?, // last_modified
                row.get::<_, Option<String>>(4)?, // expires
                row.get::<_, String>(5)?,         // content_type
                row.get::<_, u16>(6)?,            // status_code
                row.get::<_, usize>(7)?,          // original_size
                row.get::<_, usize>(8)?,          // compressed_size
                row.get::<_, String>(9)?,         // compression
                row.get::<_, String>(10)?,        // file_hash
                row.get::<_, u64>(11)?,           // timestamp
                row.get::<_, u64>(12)?,           // last_accessed
                row.get::<_, u32>(13)?,           // access_count
            ))
        });

        match result {
            Ok((
                url,
                etag,
                cache_control,
                last_modified,
                expires,
                content_type,
                status_code,
                original_size,
                compressed_size,
                compression,
                file_hash,
                timestamp,
                last_accessed,
                access_count,
            )) => {
                // Read file from disk
                let file_path = self.content_dir.join(&file_hash);
                let data = std::fs::read(&file_path)?;

                // Update last_accessed and access_count
                let _ = conn.execute(
                    "UPDATE cache_entries SET last_accessed = ?1, access_count = ?2 WHERE url = ?3",
                    params![
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        access_count + 1,
                        url
                    ],
                );

                let entry = CacheEntry {
                    url,
                    etag,
                    cache_control,
                    last_modified,
                    expires,
                    content_type,
                    status_code,
                    original_size,
                    compressed_size,
                    compression: CompressionMethod::from_str(&compression),
                    timestamp: UNIX_EPOCH + std::time::Duration::from_secs(timestamp),
                    last_accessed: UNIX_EPOCH + std::time::Duration::from_secs(last_accessed),
                    access_count,
                };

                Ok(Some((entry, data)))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }

    /// Store resource in cache
    pub fn set(
        &self,
        url: &str,
        data: &[u8],
        etag: Option<String>,
        cache_control: Option<String>,
        last_modified: Option<String>,
        expires: Option<String>,
        content_type: String,
        status_code: u16,
        compression: CompressionMethod,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let compressed_data = compress_data(data, compression)?;
        let original_size = data.len();
        let compressed_size = compressed_data.len();

        // Generate hash for file storage
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        hasher.update(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_nanos()
                .to_le_bytes(),
        );
        let digest = hasher.finalize();
        let file_hash = hex_encode(&digest);

        // Write compressed data to disk
        let file_path = self.content_dir.join(&file_hash);
        std::fs::write(&file_path, &compressed_data)?;

        // Store metadata in database
        let conn = self.conn.lock().unwrap();
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        conn.execute(
            "INSERT OR REPLACE INTO cache_entries 
             (url, etag, cache_control, last_modified, expires, content_type, 
              status_code, original_size, compressed_size, compression, file_hash, 
              timestamp, last_accessed, access_count) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                url,
                etag,
                cache_control,
                last_modified,
                expires,
                content_type,
                status_code,
                original_size,
                compressed_size,
                compression.as_str(),
                file_hash,
                now,
                now,
                0u32, // access_count starts at 0
            ],
        )?;

        // Check if we need to evict entries due to size
        self.evict_if_needed(&conn)?;

        Ok(())
    }

    /// Check if entry exists and is valid
    pub fn exists(&self, url: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let exists = conn.query_row("SELECT 1 FROM cache_entries WHERE url = ?1", [url], |_| {
            Ok(())
        });

        Ok(exists.is_ok())
    }

    /// Clear all cache
    pub fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();

        // Delete all database entries
        conn.execute("DELETE FROM cache_entries", [])?;

        // Delete all files in content directory
        for entry in std::fs::read_dir(&self.content_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                std::fs::remove_file(path)?;
            }
        }

        Ok(())
    }

    /// Clear entries older than specified days
    pub fn clear_old_entries(&self, days: u64) -> Result<u64, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let cutoff_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() - (days * 86400);

        // Get files to delete
        let mut stmt = conn.prepare("SELECT file_hash FROM cache_entries WHERE timestamp < ?1")?;

        let file_hashes: Vec<String> = stmt
            .query_map([cutoff_time], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        // Delete database entries
        let deleted = conn.execute(
            "DELETE FROM cache_entries WHERE timestamp < ?1",
            [cutoff_time],
        )? as u64;

        // Delete files
        for hash in file_hashes {
            let path = self.content_dir.join(&hash);
            let _ = std::fs::remove_file(path);
        }

        Ok(deleted)
    }

    /// Clear cache for specific origin/domain
    pub fn clear_by_origin(&self, origin: &str) -> Result<u64, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();

        // Get files to delete (all URLs matching origin)
        let mut stmt = conn.prepare("SELECT file_hash FROM cache_entries WHERE url LIKE ?1")?;

        let pattern = format!("{}%", origin);
        let file_hashes: Vec<String> = stmt
            .query_map([pattern.clone()], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        // Delete database entries
        let deleted =
            conn.execute("DELETE FROM cache_entries WHERE url LIKE ?1", [pattern])? as u64;

        // Delete files
        for hash in file_hashes {
            let path = self.content_dir.join(&hash);
            let _ = std::fs::remove_file(path);
        }

        Ok(deleted)
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> Result<(u64, u32, u64), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*), SUM(compressed_size) FROM cache_entries")?;

        let (count, total_size): (u32, Option<u64>) =
            stmt.query_row([], |row| Ok((row.get(0)?, row.get(1)?)))?;

        Ok((total_size.unwrap_or(0), count, self.max_size_bytes))
    }

    fn evict_if_needed(&self, conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
        // Check total size
        let mut stmt = conn.prepare("SELECT SUM(compressed_size) FROM cache_entries")?;
        let total_size: u64 =
            stmt.query_row([], |row| Ok(row.get::<_, Option<u64>>(0)?.unwrap_or(0)))?;

        if total_size > self.max_size_bytes {
            // Evict LRU entries until we're under the limit
            let target_size = (self.max_size_bytes as f64 * 0.8) as u64; // Target 80% of max

            let mut stmt = conn.prepare(
                "SELECT file_hash, compressed_size FROM cache_entries 
                 ORDER BY last_accessed ASC",
            )?;

            let to_delete: Vec<(String, u64)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .filter_map(|r| r.ok())
                .collect();

            let mut current_size = total_size;
            for (file_hash, size) in to_delete {
                if current_size <= target_size {
                    break;
                }

                // Delete from database
                conn.execute(
                    "DELETE FROM cache_entries WHERE file_hash = ?1",
                    [file_hash.clone()],
                )?;

                // Delete file
                let path = self.content_dir.join(&file_hash);
                let _ = std::fs::remove_file(path);

                current_size = current_size.saturating_sub(size);
            }
        }

        Ok(())
    }
}

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
