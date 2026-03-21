use crate::runtime::core::event_loop::IDBEventMessage;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

/// Comandos para o Worker de IndexedDB - Especificação Completa
pub enum IDBWorkerCommand {
    OpenDb {
        request_callback_id: usize,
        name: String,
        version: Option<u32>,
        origin: String,
        db_path: PathBuf,
    },
    // Transações
    TransactionStart {
        db_name: String,
        store_names: Vec<String>,
        mode: String,
        transaction_id: usize,
    },
    TransactionCommit {
        transaction_id: usize,
    },
    TransactionAbort {
        transaction_id: usize,
    },
    // Object Stores
    CreateObjectStore {
        transaction_id: usize,
        name: String,
        key_path: Option<String>,
        auto_increment: bool,
    },
    DeleteObjectStore {
        transaction_id: usize,
        name: String,
    },
    // Operações de Dados
    StorePut {
        request_callback_id: usize,
        transaction_id: usize,
        store_name: String,
        key: String,
        value_json: String,
        overwrite: bool,
    },
    StoreGet {
        request_callback_id: usize,
        transaction_id: usize,
        store_name: String,
        key: String,
    },
    StoreDelete {
        request_callback_id: usize,
        transaction_id: usize,
        store_name: String,
        key: String,
    },
    StoreClear {
        request_callback_id: usize,
        transaction_id: usize,
        store_name: String,
    },
    // Índices
    CreateIndex {
        transaction_id: usize,
        store_name: String,
        index_name: String,
        key_path: String,
        unique: bool,
    },
    // Cursores
    OpenCursor {
        request_callback_id: usize,
        transaction_id: usize,
        store_name: String,
        index_name: Option<String>,
        range: Option<String>,
        direction: String,
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
            let mut connections: HashMap<String, Connection> = HashMap::new();
            let mut active_transactions: HashMap<usize, (String, String)> = HashMap::new();

            while let Some(command) = rx.blocking_recv() {
                match command {
                    IDBWorkerCommand::OpenDb {
                        request_callback_id,
                        name,
                        version,
                        origin,
                        db_path,
                    } => {
                        let conn_key = format!("{}:{}", origin, name);

                        let result = (|| -> SqliteResult<(u32, u32)> {
                            let conn = Connection::open(&db_path)?;

                            conn.execute_batch(
                                "CREATE TABLE IF NOT EXISTS idb_metadata (key TEXT PRIMARY KEY, value TEXT);
                                 CREATE TABLE IF NOT EXISTS idb_object_stores (
                                     name TEXT PRIMARY KEY, 
                                     key_path TEXT, 
                                     auto_increment BOOLEAN
                                 );
                                 CREATE TABLE IF NOT EXISTS idb_indices (
                                     name TEXT, 
                                     store_name TEXT, 
                                     key_path TEXT, 
                                     unique_idx BOOLEAN, 
                                     PRIMARY KEY (name, store_name)
                                 );"
                            )?;

                            let current_version: u32 = conn
                                .query_row(
                                    "SELECT value FROM idb_metadata WHERE key = 'version'",
                                    [],
                                    |row| row.get::<_, String>(0),
                                )
                                .and_then(|v| {
                                    v.parse::<u32>().map_err(|_| rusqlite::Error::InvalidQuery)
                                })
                                .unwrap_or(0);

                            let target_version = version.unwrap_or(if current_version == 0 {
                                1
                            } else {
                                current_version
                            });

                            connections.insert(conn_key.clone(), conn);
                            if target_version > current_version {
                                Ok((current_version, target_version))
                            } else {
                                Ok((current_version, 0))
                            }
                        })();

                        match result {
                            Ok((old_v, new_v)) => {
                                if new_v > 0 {
                                    let sp_name =
                                        format!("sp_upgradeneeded_{}", request_callback_id);
                                    if let Some(conn) =
                                        connections.get_mut(&format!("{}:{}", origin, name))
                                    {
                                        let _ = conn.execute(&format!("SAVEPOINT {}", sp_name), []);
                                        active_transactions
                                            .insert(1, (format!("{}:{}", origin, name), sp_name));
                                    }
                                    let _ = el_tx.send(IDBEventMessage::UpgradeNeeded {
                                        request_callback_id,
                                        transaction_id: 1,
                                        db_name: name.clone(),
                                        old_version: old_v,
                                        new_version: new_v,
                                    });
                                } else {
                                    let _ = el_tx.send(IDBEventMessage::DatabaseSuccess {
                                        callback_id: request_callback_id,
                                        db_name: name.clone(),
                                        version: old_v,
                                    });
                                }
                            }
                            Err(e) => {
                                let _ = el_tx.send(IDBEventMessage::Error {
                                    callback_id: request_callback_id,
                                    error_name: "OpenFailed".into(),
                                    error_message: e.to_string(),
                                });
                            }
                        }
                    }

                    IDBWorkerCommand::TransactionStart {
                        db_name,
                        store_names: _,
                        mode: _,
                        transaction_id,
                    } => {
                        if let Some((conn_key, conn)) =
                            connections.iter_mut().find(|(k, _)| k.contains(&db_name))
                        {
                            let sp_name = format!("sp_tx_{}", transaction_id);
                            let _ = conn.execute(&format!("SAVEPOINT {}", sp_name), []);
                            active_transactions.insert(transaction_id, (conn_key.clone(), sp_name));
                        }
                    }

                    IDBWorkerCommand::TransactionCommit { transaction_id } => {
                        if let Some((conn_key, sp_name)) =
                            active_transactions.remove(&transaction_id)
                        {
                            if let Some(conn) = connections.get_mut(&conn_key) {
                                let _ = conn.execute(&format!("RELEASE SAVEPOINT {}", sp_name), []);
                            }
                        }
                    }

                    IDBWorkerCommand::TransactionAbort { transaction_id } => {
                        if let Some((conn_key, sp_name)) =
                            active_transactions.remove(&transaction_id)
                        {
                            if let Some(conn) = connections.get_mut(&conn_key) {
                                let _ =
                                    conn.execute(&format!("ROLLBACK TO SAVEPOINT {}", sp_name), []);
                            }
                        }
                    }

                    IDBWorkerCommand::StorePut {
                        request_callback_id,
                        transaction_id,
                        store_name,
                        key,
                        value_json,
                        overwrite,
                    } => {
                        if let Some((conn_key, _)) = active_transactions.get(&transaction_id) {
                            if let Some(conn) = connections.get_mut(conn_key) {
                                let table_name = format!("idb_data_{}", store_name);
                                let sql = if overwrite {
                                    format!(
                                        "INSERT OR REPLACE INTO {} (key, value) VALUES (?1, ?2)",
                                        table_name
                                    )
                                } else {
                                    format!(
                                        "INSERT INTO {} (key, value) VALUES (?1, ?2)",
                                        table_name
                                    )
                                };
                                match conn.execute(&sql, params![key, value_json]) {
                                    Ok(_) => {
                                        let _ = el_tx.send(IDBEventMessage::Success {
                                            callback_id: request_callback_id,
                                            result_json: format!(r#""{}""#, key),
                                        });
                                    }
                                    Err(e) => {
                                        let _ = el_tx.send(IDBEventMessage::Error {
                                            callback_id: request_callback_id,
                                            error_name: "ConstraintError".into(),
                                            error_message: e.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }

                    IDBWorkerCommand::CreateObjectStore {
                        transaction_id,
                        name,
                        key_path,
                        auto_increment,
                    } => {
                        if let Some((conn_key, _)) = active_transactions.get(&transaction_id) {
                            if let Some(conn) = connections.get_mut(conn_key) {
                                let table_name = format!("idb_data_{}", name);
                                let _ = conn.execute(&format!(
                                    "CREATE TABLE IF NOT EXISTS {} (key TEXT PRIMARY KEY, value TEXT)", table_name
                                ), []);
                                let _ = conn.execute(
                                    "INSERT OR REPLACE INTO idb_object_stores (name, key_path, auto_increment) VALUES (?1, ?2, ?3)",
                                    params![name, key_path, auto_increment]
                                );
                            }
                        }
                    }

                    IDBWorkerCommand::DeleteObjectStore {
                        transaction_id,
                        name,
                    } => {
                        if let Some((conn_key, _)) = active_transactions.get(&transaction_id) {
                            if let Some(conn) = connections.get_mut(conn_key) {
                                let table_name = format!("idb_data_{}", name);
                                let _ = conn
                                    .execute(&format!("DROP TABLE IF EXISTS {}", table_name), []);
                                let _ = conn.execute(
                                    "DELETE FROM idb_object_stores WHERE name = ?1",
                                    params![name],
                                );
                            }
                        }
                    }

                    IDBWorkerCommand::StoreGet {
                        request_callback_id,
                        transaction_id,
                        store_name,
                        key,
                    } => {
                        if let Some((conn_key, _)) = active_transactions.get(&transaction_id) {
                            if let Some(conn) = connections.get(conn_key) {
                                let table_name = format!("idb_data_{}", store_name);
                                let sql =
                                    format!("SELECT value FROM {} WHERE key = ?1", table_name);
                                let result: SqliteResult<String> =
                                    conn.query_row(&sql, params![key], |row| row.get(0));
                                match result {
                                    Ok(val) => {
                                        let _ = el_tx.send(IDBEventMessage::Success {
                                            callback_id: request_callback_id,
                                            result_json: val,
                                        });
                                    }
                                    Err(_) => {
                                        let _ = el_tx.send(IDBEventMessage::Success {
                                            callback_id: request_callback_id,
                                            result_json: "undefined".into(),
                                        });
                                    }
                                }
                            }
                        }
                    }

                    IDBWorkerCommand::StoreDelete {
                        request_callback_id,
                        transaction_id,
                        store_name,
                        key,
                    } => {
                        if let Some((conn_key, _)) = active_transactions.get(&transaction_id) {
                            if let Some(conn) = connections.get_mut(conn_key) {
                                let table_name = format!("idb_data_{}", store_name);
                                let sql = format!("DELETE FROM {} WHERE key = ?1", table_name);
                                let _ = conn.execute(&sql, params![key]);
                                let _ = el_tx.send(IDBEventMessage::Success {
                                    callback_id: request_callback_id,
                                    result_json: "undefined".into(),
                                });
                            }
                        }
                    }

                    IDBWorkerCommand::StoreClear {
                        request_callback_id,
                        transaction_id,
                        store_name,
                    } => {
                        if let Some((conn_key, _)) = active_transactions.get(&transaction_id) {
                            if let Some(conn) = connections.get_mut(conn_key) {
                                let table_name = format!("idb_data_{}", store_name);
                                let _ = conn.execute(&format!("DELETE FROM {}", table_name), []);
                                let _ = el_tx.send(IDBEventMessage::Success {
                                    callback_id: request_callback_id,
                                    result_json: "undefined".into(),
                                });
                            }
                        }
                    }

                    IDBWorkerCommand::CreateIndex {
                        transaction_id,
                        store_name,
                        index_name,
                        key_path,
                        unique,
                    } => {
                        if let Some((conn_key, _)) = active_transactions.get(&transaction_id) {
                            if let Some(conn) = connections.get_mut(conn_key) {
                                let table_name = format!("idb_idx_{}_{}", store_name, index_name);
                                let _ = conn.execute(&format!(
                                    "CREATE TABLE IF NOT EXISTS {} (idx_key TEXT, record_key TEXT, PRIMARY KEY (idx_key, record_key))", table_name
                                ), []);
                                let _ = conn.execute(
                                    "INSERT OR REPLACE INTO idb_indices (name, store_name, key_path, unique_idx) VALUES (?1, ?2, ?3, ?4)",
                                    params![index_name, store_name, key_path, unique]
                                );
                            }
                        }
                    }

                    IDBWorkerCommand::OpenCursor {
                        request_callback_id,
                        transaction_id,
                        store_name,
                        index_name: _,
                        range: _,
                        direction: _,
                    } => {
                        if let Some((conn_key, _)) = active_transactions.get(&transaction_id) {
                            if let Some(conn) = connections.get(conn_key) {
                                let table_name = format!("idb_data_{}", store_name);
                                match conn.prepare(&format!(
                                    "SELECT key, value FROM {} ORDER BY key",
                                    table_name
                                )) {
                                    Ok(mut stmt) => {
                                        let rows = stmt.query_map([], |row| {
                                            Ok(format!(
                                                r#"{{"key": "{}", "value": {}}}"#,
                                                row.get::<_, String>(0)?,
                                                row.get::<_, String>(1)?
                                            ))
                                        });
                                        if let Ok(rows) = rows {
                                            let mut results = Vec::new();
                                            for row in rows {
                                                if let Ok(json) = row {
                                                    results.push(json);
                                                }
                                            }
                                            let _ = el_tx.send(IDBEventMessage::Success {
                                                callback_id: request_callback_id,
                                                result_json: format!("[{}]", results.join(",")),
                                            });
                                        }
                                    }
                                    Err(_) => {}
                                }
                            }
                        }
                    }
                }
            }
        });

        Self { tx }
    }
}
