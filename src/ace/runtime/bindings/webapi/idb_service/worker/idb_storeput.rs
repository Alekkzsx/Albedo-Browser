{
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
