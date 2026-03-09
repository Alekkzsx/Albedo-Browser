use rusqlite::{Connection, Result as SqliteResult, params};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwRegistrationData {
    pub id: String,
    pub scope: String,
    pub script_url: String,
    pub origin: String,
    pub last_update_check: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwSyncTaskData {
    pub id: String,
    pub tag: String,
    pub registration_id: String,
    pub created_at: u64,
    pub retry_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwCacheEntryData {
    pub id: String,
    pub cache_name: String,
    pub origin: String,
    pub url: String,
    pub status: u16,
    pub headers: String, // JSON
    pub body: Vec<u8>,
    pub created_at: u64,
}

pub struct ServiceWorkerDatabase {
    conn: Arc<Mutex<Connection>>,
}

impl ServiceWorkerDatabase {
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
            CREATE INDEX IF NOT EXISTS idx_sw_cache_entries_url ON sw_cache_entries(url);"
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    // --- Registrations ---

    pub fn save_registration(&self, reg: &SwRegistrationData) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO sw_registrations (id, scope, script_url, origin, last_update_check) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![reg.id, reg.scope, reg.script_url, reg.origin, reg.last_update_check],
        )?;
        Ok(())
    }

    pub fn get_all_registrations(&self) -> SqliteResult<Vec<SwRegistrationData>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, scope, script_url, origin, last_update_check FROM sw_registrations")?;
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

    pub fn get_registrations_for_origin(&self, origin: &str) -> SqliteResult<Vec<SwRegistrationData>> {
        let conn = self.conn.lock().unwrap();
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

    pub fn delete_registration(&self, id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM sw_registrations WHERE id = ?1", params![id])?;
        Ok(())
    }

    // --- Sync Tasks ---

    pub fn save_sync_task(&self, task: &SwSyncTaskData) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO sw_sync_tasks (id, tag, registration_id, created_at, retry_count) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![task.id, task.tag, task.registration_id, task.created_at, task.retry_count],
        )?;
        Ok(())
    }

    pub fn get_pending_sync_tasks(&self) -> SqliteResult<Vec<SwSyncTaskData>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, tag, registration_id, created_at, retry_count FROM sw_sync_tasks")?;
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

    pub fn delete_sync_task(&self, id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM sw_sync_tasks WHERE id = ?1", params![id])?;
        Ok(())
    }

    // --- Cache API ---

    pub fn save_cache(&self, name: String, origin: String) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        conn.execute(
            "INSERT OR IGNORE INTO sw_caches (id, name, origin, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, origin, now],
        )?;
        Ok(())
    }

    pub fn open_cache(&self, name: &str, origin: &str) -> SqliteResult<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id FROM sw_caches WHERE name = ?1 AND origin = ?2",
            params![name, origin],
            |row| row.get(0),
        )
    }

    pub fn save_cache_entry(&self, entry: &SwCacheEntryData) -> SqliteResult<()> {
        let cache_id = self.open_cache(&entry.cache_name, &entry.origin)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO sw_cache_entries (id, cache_id, url, method, status, headers, body, timestamp) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![entry.id, cache_id, entry.url, "GET", entry.status, entry.headers, entry.body, entry.created_at],
        )?;
        Ok(())
    }

    pub fn get_cache_entry(&self, cache_name: &str, url: &str) -> SqliteResult<Option<SwCacheEntryData>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT e.id, e.status, e.headers, e.body, e.timestamp, c.origin
             FROM sw_cache_entries e
             JOIN sw_caches c ON e.cache_id = c.id
             WHERE c.name = ?1 AND e.url = ?2"
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

    pub fn delete_cache_entry(&self, cache_name: &str, url: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM sw_cache_entries WHERE url = ?2 AND cache_id IN (SELECT id FROM sw_caches WHERE name = ?1)",
            params![cache_name, url],
        )?;
        Ok(())
    }
}
