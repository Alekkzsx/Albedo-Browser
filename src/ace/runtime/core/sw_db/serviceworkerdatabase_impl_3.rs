use super::*;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};



impl ServiceWorkerDatabase {

    /// TODO: add docs
    pub fn delete_cache_entry(&self, cache_name: &str, url: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "DELETE FROM sw_cache_entries WHERE url = ?2 AND cache_id IN (SELECT id FROM sw_caches WHERE name = ?1)",
            params![cache_name, url],
        )?;
        Ok(())
    }
}
