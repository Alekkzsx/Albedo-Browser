use std::sync::mpsc::Sender;
use rusqlite::{Connection, Result as SqliteResult};
use crate::runtime::core::event_loop::IDBEventMessage;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

pub enum IDBWorkerCommand {
    OpenDb {
        request_callback_id: usize,
        name: String,
        version: Option<u32>,
        origin: String,
        db_path: PathBuf,
    },
    StorePut {
        request_callback_id: usize,
        db_name: String,
        store_name: String,
        key: String,
        value_json: String,
    },
    StoreGet {
        request_callback_id: usize,
        db_name: String,
        store_name: String,
        key: String,
    },
    StoreDelete {
        request_callback_id: usize,
        db_name: String,
        store_name: String,
        key: String,
    },
    StoreClear {
        request_callback_id: usize,
        db_name: String,
        store_name: String,
    },
}

pub struct IDBServiceWorker {
    pub tx: tokio::sync::mpsc::UnboundedSender<IDBWorkerCommand>,
}

impl IDBServiceWorker {
    pub fn new(event_loop_tx: Sender<IDBEventMessage>) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<IDBWorkerCommand>();
        let el_tx = event_loop_tx.clone();

        tokio::task::spawn_blocking(move || {
            // Keep connections open per origin/name
            let mut connections: HashMap<String, Connection> = HashMap::new();

            while let Some(command) = rx.blocking_recv() {
                match command {
                    IDBWorkerCommand::OpenDb { request_callback_id, name, version, origin, db_path } => {
                        let conn_key = format!("{}:{}", origin, name);
                        
                        let setup_conn = || -> SqliteResult<u32> {
                            let mut conn = Connection::open(&db_path)?;
                            
                            // Create underlying master tables if they don't exist
                            conn.execute(
                                "CREATE TABLE IF NOT EXISTS databases (
                                    name TEXT PRIMARY KEY,
                                    version INTEGER NOT NULL
                                )",
                                (),
                            )?;

                            conn.execute(
                                "CREATE TABLE IF NOT EXISTS object_stores (
                                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                                    db_name TEXT NOT NULL,
                                    name TEXT NOT NULL,
                                    key_path TEXT,
                                    auto_increment BOOLEAN NOT NULL,
                                    UNIQUE(db_name, name)
                                )",
                                (),
                            )?;
                            
                            conn.execute(
                                "CREATE TABLE IF NOT EXISTS records (
                                    os_id INTEGER NOT NULL,
                                    key_text TEXT NOT NULL,
                                    value_json TEXT NOT NULL,
                                    PRIMARY KEY (os_id, key_text)
                                )",
                                (),
                            )?;

                            let current_version: Option<u32> = conn.query_row(
                                "SELECT version FROM databases WHERE name = ?1",
                                [&name],
                                |row| row.get(0),
                            ).optional()?;

                            let current_v = current_version.unwrap_or(0);
                            let target_v = version.unwrap_or(if current_v == 0 { 1 } else { current_v });

                            if target_v > current_v {
                                // Save new version
                                conn.execute(
                                    "INSERT OR REPLACE INTO databases (name, version) VALUES (?1, ?2)",
                                    rusqlite::params![name, target_v],
                                )?;
                                
                                Ok(target_v) // Return new version indicating Upgrade Needed
                            } else if target_v < current_v {
                                Err(rusqlite::Error::InvalidQuery) // Version downgrade not allowed
                            } else {
                                Ok(0) // 0 implies no upgrade needed
                            }
                        };

                        match setup_conn() {
                            Ok(upgrade_version) => {
                                if upgrade_version > 0 {
                                    // Spec says we fire onupgradeneeded first
                                    let _ = el_tx.send(IDBEventMessage::UpgradeNeeded {
                                        request_callback_id,
                                        transaction_id: 0, // Mock id for now
                                        old_version: 0,
                                        new_version: upgrade_version,
                                    });
                                } else {
                                    // Just success
                                    // For simplicity, result of Open success is the db object mapping id.
                                    // Let's pass a JSON structure identifying the connection.
                                    let result = format!(r#"{{"name": "{}", "version": "{}"}}"#, name, version.unwrap_or(1));
                                    let _ = el_tx.send(IDBEventMessage::Success {
                                        callback_id: request_callback_id,
                                        result_json: result,
                                    });
                                }
                            },
                            Err(e) => {
                                let _ = el_tx.send(IDBEventMessage::Error {
                                    callback_id: request_callback_id,
                                    error_name: "UnknownError".to_string(),
                                    error_message: e.to_string(),
                                });
                            }
                        }
                    },
                    IDBWorkerCommand::StorePut { request_callback_id, db_name, store_name, key, value_json } => {
                        // In a real app, finding the exact connection from pooling.
                        // Here we just map by origin:name hackily or reopen. We'll reopen for simplicity in MVP.
                        // A true implementation needs `origin` tied to the worker message.
                        // For MVP, we presume db_name maps to the local sqlite file.
                        let mut db_path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
                        db_path.push("albedo");
                        // We lack origin here! For MVP, let's just use `db_name.sqlite`.
                        db_path.push(format!("indexeddb_{}.sqlite", db_name.replace("://", "_").replace(":", "_")));
                        
                        match Connection::open(&db_path) {
                            Ok(conn) => {
                                // Needs OS_ID
                                let os_id_query: SqliteResult<u32> = conn.query_row(
                                    "SELECT id FROM object_stores WHERE db_name = ?1 AND name = ?2",
                                    rusqlite::params![db_name, store_name],
                                    |row| row.get(0)
                                );
                                
                                match os_id_query {
                                    Ok(os_id) => {
                                        let insert = conn.execute(
                                            "INSERT OR REPLACE INTO records (os_id, key_text, value_json) VALUES (?1, ?2, ?3)",
                                            rusqlite::params![os_id, key, value_json]
                                        );
                                        if insert.is_ok() {
                                            let _ = el_tx.send(IDBEventMessage::Success {
                                                callback_id: request_callback_id,
                                                result_json: format!(r#""{}""#, key),
                                            });
                                        } else {
                                            let _ = el_tx.send(IDBEventMessage::Error { callback_id: request_callback_id, error_name: "DataError".into(), error_message: "Insert failed".into() });
                                        }
                                    },
                                    Err(_) => {
                                        let _ = el_tx.send(IDBEventMessage::Error { callback_id: request_callback_id, error_name: "NotFoundError".into(), error_message: "ObjectStore not found".into() });
                                    }
                                }
                            },
                            Err(_) => {
                                let _ = el_tx.send(IDBEventMessage::Error { callback_id: request_callback_id, error_name: "DatabaseError".into(), error_message: "DB missing".into() });
                            }
                        }
                    },
                    IDBWorkerCommand::StoreGet { request_callback_id, db_name, store_name, key } => {
                        let mut db_path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
                        db_path.push("albedo");
                        db_path.push(format!("indexeddb_{}.sqlite", db_name.replace("://", "_").replace(":", "_")));
                        
                        if let Ok(conn) = Connection::open(&db_path) {
                            let val_query: SqliteResult<String> = conn.query_row(
                                "SELECT value_json FROM records r JOIN object_stores os ON r.os_id = os.id WHERE os.db_name = ?1 AND os.name = ?2 AND r.key_text = ?3",
                                rusqlite::params![db_name, store_name, key],
                                |row| row.get(0)
                            );
                            
                            match val_query {
                                Ok(json) => {
                                    let _ = el_tx.send(IDBEventMessage::Success { callback_id: request_callback_id, result_json: json });
                                },
                                Err(_) => {
                                    let _ = el_tx.send(IDBEventMessage::Success { callback_id: request_callback_id, result_json: "null".into() }); // Not found is success=null in IDB
                                }
                            }
                        } else {
                            let _ = el_tx.send(IDBEventMessage::Error { callback_id: request_callback_id, error_name: "DatabaseError".into(), error_message: "DB missing".into() });
                        }
                    },
                    IDBWorkerCommand::StoreDelete { request_callback_id, db_name, store_name, key } => {
                        let mut db_path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
                        db_path.push("albedo");
                        db_path.push(format!("indexeddb_{}.sqlite", db_name.replace("://", "_").replace(":", "_")));
                        if let Ok(conn) = Connection::open(&db_path) {
                            let _ = conn.execute(
                                "DELETE FROM records WHERE os_id IN (SELECT id FROM object_stores WHERE db_name = ?1 AND name = ?2) AND key_text = ?3",
                                rusqlite::params![db_name, store_name, key]
                            );
                            let _ = el_tx.send(IDBEventMessage::Success { callback_id: request_callback_id, result_json: "undefined".into() });
                        }
                    },
                    IDBWorkerCommand::StoreClear { request_callback_id, db_name, store_name } => {
                        let mut db_path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
                        db_path.push("albedo");
                        db_path.push(format!("indexeddb_{}.sqlite", db_name.replace("://", "_").replace(":", "_")));
                        if let Ok(conn) = Connection::open(&db_path) {
                            let _ = conn.execute(
                                "DELETE FROM records WHERE os_id IN (SELECT id FROM object_stores WHERE db_name = ?1 AND name = ?2)",
                                rusqlite::params![db_name, store_name]
                            );
                            let _ = el_tx.send(IDBEventMessage::Success { callback_id: request_callback_id, result_json: "undefined".into() });
                        }
                    }
                }
            }
        });

        Self { tx }
    }
}

// trait extension for Option
trait OptionalExt<T> {
    fn optional(self) -> SqliteResult<Option<T>>;
}
impl<T> OptionalExt<T> for SqliteResult<T> {
    fn optional(self) -> SqliteResult<Option<T>> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
