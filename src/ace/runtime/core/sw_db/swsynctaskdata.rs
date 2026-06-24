use super::*;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};



#[derive(Clone, Debug)]
pub struct SwSyncTaskData {
    pub id: String,
    pub tag: String,
    pub registration_id: String,
    pub created_at: u64,
    pub retry_count: u32,
}
