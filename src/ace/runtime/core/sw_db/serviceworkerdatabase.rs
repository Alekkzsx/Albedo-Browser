use super::*;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};



pub struct ServiceWorkerDatabase {
    conn: Arc<Mutex<Connection>>,
}
