use super::*;
use crate::utils::hex::encode as hex_encode;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Compression method for cached resources


impl DiskCache {

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
        let mut entropy = Vec::with_capacity(url.len() + 16);
        entropy.extend_from_slice(url.as_bytes());
        entropy.extend_from_slice(
            &SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_nanos()
                .to_le_bytes(),
        );
        let digest = crate::ace::crypto::sha2::sha256(&entropy);
        let file_hash = hex_encode(&digest);

        // Write compressed data to disk
        let file_path = self.content_dir.join(&file_hash);
        std::fs::write(&file_path, &compressed_data)?;

        // Store metadata in database
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
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
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let exists = conn.query_row("SELECT 1 FROM cache_entries WHERE url = ?1", [url], |_| {
            Ok(())
        });

        Ok(exists.is_ok())
    }

    /// Clear all cache
    pub fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

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
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
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
}
