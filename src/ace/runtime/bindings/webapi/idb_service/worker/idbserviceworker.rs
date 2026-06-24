use super::*;
use crate::ace::runtime::core::event_loop::IDBEventMessage;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

/// Comandos para o Worker de IndexedDB - Especificação Completa


pub struct IDBServiceWorker {
    pub tx: tokio::sync::mpsc::UnboundedSender<IDBWorkerCommand>,
}
