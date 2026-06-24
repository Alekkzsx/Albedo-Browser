use super::*;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};



impl ServiceWorkerDatabase {

    /// TODO: add docs
    pub fn delete_registration(&self, id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute("DELETE FROM sw_registrations WHERE id = ?1", params![id])?;
        Ok(())
    }

    // --- Sync Tasks ---

    /// TODO: add docs
    pub fn save_sync_task(&self, task: &SwSyncTaskData) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO sw_sync_tasks (id, tag, registration_id, created_at, retry_count) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![task.id, task.tag, task.registration_id, task.created_at, task.retry_count],
        )?;
        Ok(())
    }

    /// TODO: add docs
    pub fn get_pending_sync_tasks(&self) -> SqliteResult<Vec<SwSyncTaskData>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT id, tag, registration_id, created_at, retry_count FROM sw_sync_tasks",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SwSyncTaskData {
                id: row.get(0)?,
                tag: row.get(1)?,
                registration_id: row.get(2)?,
                created_at: row.get(3)?,
                retry_count: row.get(4)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// TODO: add docs
    pub fn delete_sync_task(&self, id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute("DELETE FROM sw_sync_tasks WHERE id = ?1", params![id])?;
        Ok(())
    }

    // --- Cache API ---

    /// TODO: add docs
    pub fn save_cache(&self, name: String, origin: String) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let id = crate::utils::uuid::Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Albedo Engine: internal invariant violated")
            .as_secs();
        conn.execute(
            "INSERT OR IGNORE INTO sw_caches (id, name, origin, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, origin, now],
        )?;
        Ok(())
    }

    /// TODO: add docs
    pub fn open_cache(&self, name: &str, origin: &str) -> SqliteResult<String> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.query_row(
            "SELECT id FROM sw_caches WHERE name = ?1 AND origin = ?2",
            params![name, origin],
            |row| row.get(0),
        )
    }

    /// TODO: add docs
    pub fn save_cache_entry(&self, entry: &SwCacheEntryData) -> SqliteResult<()> {
        let cache_id = self.open_cache(&entry.cache_name, &entry.origin)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO sw_cache_entries (id, cache_id, url, method, status, headers, body, timestamp) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![entry.id, cache_id, entry.url, "GET", entry.status, entry.headers, entry.body, entry.created_at],
        )?;
        Ok(())
    }

    /// TODO: add docs
    pub fn get_cache_entry(
        &self,
        cache_name: &str,
        url: &str,
    ) -> SqliteResult<Option<SwCacheEntryData>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT e.id, e.status, e.headers, e.body, e.timestamp, c.origin
             FROM sw_cache_entries e
             JOIN sw_caches c ON e.cache_id = c.id
             WHERE c.name = ?1 AND e.url = ?2",
        )?;

        let result = stmt.query_row(params![cache_name, url], |row| {
            Ok(SwCacheEntryData {
                id: row.get(0)?,
                cache_name: cache_name.to_string(),
                status: row.get(1)?,
                headers: row.get(2)?,
                body: row.get(3)?,
                created_at: row.get(4)?,
                origin: row.get(5)?,
                url: url.to_string(),
            })
        });

        match result {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
