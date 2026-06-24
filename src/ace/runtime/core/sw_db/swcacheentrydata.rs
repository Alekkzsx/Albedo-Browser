use super::*;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};



#[derive(Clone, Debug)]
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
