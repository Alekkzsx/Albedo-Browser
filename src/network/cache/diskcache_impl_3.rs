use super::*;
use crate::utils::hex::encode as hex_encode;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Compression method for cached resources


impl DiskCache {

    /// Clear cache for specific origin/domain
    pub fn clear_by_origin(&self, origin: &str) -> Result<u64, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

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
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare("SELECT COUNT(*), SUM(compressed_size) FROM cache_entries")?;

        let (count, total_size): (u32, Option<u64>) =
            stmt.query_row([], |row| Ok((row.get(0)?, row.get(1)?)))?;

        Ok((total_size.unwrap_or(0), count, self.max_size_bytes))
    }

pub(crate) fn evict_if_needed(&self, conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
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
