use super::*;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};



impl ServiceWorkerDatabase {
    /// TODO: add docs
    pub fn new(db_path: PathBuf) -> SqliteResult<Self> {
        let conn = Connection::open(db_path)?;

        // Initialize tables
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sw_registrations (
                id TEXT PRIMARY KEY,
                scope TEXT NOT NULL,
                script_url TEXT NOT NULL,
                origin TEXT NOT NULL,
                last_update_check INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sw_sync_tasks (
                id TEXT PRIMARY KEY,
                tag TEXT NOT NULL,
                registration_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                retry_count INTEGER NOT NULL,
                FOREIGN KEY(registration_id) REFERENCES sw_registrations(id)
            );
            CREATE TABLE IF NOT EXISTS sw_caches (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                origin TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE(name, origin)
            );
            CREATE TABLE IF NOT EXISTS sw_cache_entries (
                id TEXT PRIMARY KEY,
                cache_id TEXT NOT NULL,
                url TEXT NOT NULL,
                method TEXT NOT NULL,
                status INTEGER NOT NULL,
                headers TEXT NOT NULL,
                body BLOB,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY(cache_id) REFERENCES sw_caches(id)
            );
            CREATE INDEX IF NOT EXISTS idx_sw_registrations_origin ON sw_registrations(origin);
            CREATE INDEX IF NOT EXISTS idx_sw_cache_entries_cache_id ON sw_cache_entries(cache_id);
            CREATE INDEX IF NOT EXISTS idx_sw_cache_entries_url ON sw_cache_entries(url);",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    // --- Registrations ---

    /// TODO: add docs
    pub fn save_registration(&self, reg: &SwRegistrationData) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO sw_registrations (id, scope, script_url, origin, last_update_check) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![reg.id, reg.scope, reg.script_url, reg.origin, reg.last_update_check],
        )?;
        Ok(())
    }

    /// TODO: add docs
    pub fn get_all_registrations(&self) -> SqliteResult<Vec<SwRegistrationData>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT id, scope, script_url, origin, last_update_check FROM sw_registrations",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SwRegistrationData {
                id: row.get(0)?,
                scope: row.get(1)?,
                script_url: row.get(2)?,
                origin: row.get(3)?,
                last_update_check: row.get(4)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// TODO: add docs
    pub fn get_registrations_for_origin(
        &self,
        origin: &str,
    ) -> SqliteResult<Vec<SwRegistrationData>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare("SELECT id, scope, script_url, origin, last_update_check FROM sw_registrations WHERE origin = ?1")?;
        let rows = stmt.query_map(params![origin], |row| {
            Ok(SwRegistrationData {
                id: row.get(0)?,
                scope: row.get(1)?,
                script_url: row.get(2)?,
                origin: row.get(3)?,
                last_update_check: row.get(4)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }
}
