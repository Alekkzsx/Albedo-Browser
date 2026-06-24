{
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
