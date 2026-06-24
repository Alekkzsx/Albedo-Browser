use super::*;
use crate::ace::runtime::core::event_loop::IDBEventMessage;
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
