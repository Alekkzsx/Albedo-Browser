use super::*;
use crate::ace::runtime::core::event_loop::IDBEventMessage;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

/// Comandos para o Worker de IndexedDB - Especificação Completa


impl IDBServiceWorker {
    /// TODO: add docs
    pub fn new(event_loop_tx: Sender<IDBEventMessage>) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<IDBWorkerCommand>();
        let el_tx = event_loop_tx.clone();

        std::thread::spawn(move || {
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
                        include!("idb_opendb.rs");
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
                        include!("idb_storeput.rs");
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
                        include!("idb_opencursor.rs");
                    }
                }
            }
        });

        Self { tx }
    }
}
