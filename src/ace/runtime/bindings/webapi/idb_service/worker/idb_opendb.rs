{
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
