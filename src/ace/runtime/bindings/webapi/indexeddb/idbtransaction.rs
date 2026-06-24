use super::*;
use crate::ace::runtime::bindings::webapi::idb_service::worker::IDBWorkerCommand;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};



#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IDBTransaction {
    pub tx_id: usize,
    pub mode: String,
    #[qjs(skip_trace)]
    worker_tx: tokio::sync::mpsc::UnboundedSender<IDBWorkerCommand>,
    #[qjs(skip_trace)]
    observer_registry: Arc<Mutex<HashMap<usize, Persistent<Function<'static>>>>>,
}

#[rquickjs::methods]
impl IDBTransaction {
    #[qjs(rename = "objectStore")]
    pub fn object_store<'js>(&self, ctx: Ctx<'js>, name: String) -> Result<Value<'js>> {
        let store = IDBObjectStore {
            name,
            transaction_id: self.tx_id,
            worker_tx: self.worker_tx.clone(),
            observer_registry: self.observer_registry.clone(),
        };
        Class::instance(ctx, store).map(|i| i.into_value())
    }
    /// TODO: add docs
    pub fn commit(&self) {
        let _ = self.worker_tx.send(IDBWorkerCommand::TransactionCommit {
            transaction_id: self.tx_id,
        });
    }
    /// TODO: add docs
    pub fn abort(&self) {
        let _ = self.worker_tx.send(IDBWorkerCommand::TransactionAbort {
            transaction_id: self.tx_id,
        });
    }
}
