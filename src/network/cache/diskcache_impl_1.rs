use super::*;
use crate::utils::hex::encode as hex_encode;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Compression method for cached resources


impl DiskCache {
    /// Initialize disk cache with database and directories
    pub fn new(max_size_bytes: u64) -> Result<Self, Box<dyn std::error::Error>> {
        // Determine cache directory: $HOME/.cache/albedo/http_cache/
        let cache_dir = crate::utils::paths::cache_dir()
            .join("albedo")
            .join("http_cache");

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
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

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
                compression_str,
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
                            .expect("Albedo Engine: internal invariant violated")
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
                    compression: CompressionMethod::from_str(&compression_str),
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
}
