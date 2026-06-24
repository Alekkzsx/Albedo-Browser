use super::*;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};


#[derive(Clone, Debug)]
pub struct SwRegistrationData {
    pub id: String,
    pub scope: String,
    pub script_url: String,
    pub origin: String,
    pub last_update_check: u64,
}
