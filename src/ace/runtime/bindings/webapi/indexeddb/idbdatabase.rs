use super::*;
use crate::ace::runtime::bindings::webapi::idb_service::worker::IDBWorkerCommand;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};



#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IDBDatabase {
    pub name: String,
    pub version: u32,
    #[qjs(skip_trace)]
    pub worker_tx: tokio::sync::mpsc::UnboundedSender<IDBWorkerCommand>,
    #[qjs(skip_trace)]
    pub observer_registry: Arc<Mutex<HashMap<usize, Persistent<Function<'static>>>>>,
}

#[rquickjs::methods]
impl IDBDatabase {
    /// TODO: add docs
    pub fn transaction<'js>(
        &self,
        ctx: Ctx<'js>,
        _stores: Value<'js>,
        mode: rquickjs::prelude::Opt<String>,
    ) -> Result<Value<'js>> {
        let tx_id = NEXT_REQ_ID.fetch_add(1, Ordering::SeqCst);
        let m = mode.0.unwrap_or_else(|| "readonly".into());
        let _ = self.worker_tx.send(IDBWorkerCommand::TransactionStart {
            db_name: self.name.clone(),
            store_names: vec![],
            mode: m.clone(),
            transaction_id: tx_id,
        });
        let tx = IDBTransaction {
            tx_id,
            mode: m,
            worker_tx: self.worker_tx.clone(),
            observer_registry: self.observer_registry.clone(),
        };
        Class::instance(ctx, tx).map(|i| i.into_value())
    }

    #[qjs(rename = "createObjectStore")]
    pub fn create_object_store<'js>(
        &self,
        ctx: Ctx<'js>,
        name: String,
        _opt: rquickjs::prelude::Opt<Object<'js>>,
    ) -> Result<Value<'js>> {
        let _ = self.worker_tx.send(IDBWorkerCommand::CreateObjectStore {
            transaction_id: 1,
            name: name.clone(),
            key_path: None,
            auto_increment: false,
        });
        let store = IDBObjectStore {
            name,
            transaction_id: 1,
            worker_tx: self.worker_tx.clone(),
            observer_registry: self.observer_registry.clone(),
        };
        Class::instance(ctx, store).map(|i| i.into_value())
    }

    /// TODO: add docs
    pub fn close(&self) {}
}
