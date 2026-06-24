use super::*;
use crate::ace::runtime::bindings::webapi::idb_service::worker::IDBWorkerCommand;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};



#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IDBIndex {
    pub name: String,
    pub store_name: String,
    pub transaction_id: usize,
    #[qjs(skip_trace)]
    worker_tx: tokio::sync::mpsc::UnboundedSender<IDBWorkerCommand>,
    #[qjs(skip_trace)]
    observer_registry: Arc<Mutex<HashMap<usize, Persistent<Function<'static>>>>>,
}

#[rquickjs::methods]
impl IDBIndex {
    /// TODO: add docs
    pub fn get<'js>(&self, ctx: Ctx<'js>, key: String) -> Result<Value<'js>> {
        let cb_id = NEXT_REQ_ID.fetch_add(1, Ordering::SeqCst);
        let _ = self.worker_tx.send(IDBWorkerCommand::StoreGet {
            request_callback_id: cb_id,
            transaction_id: self.transaction_id,
            store_name: self.store_name.clone(),
            key,
        });
        Class::instance(
            ctx,
            IDBRequest {
                callback_id: cb_id,
                observer_registry: self.observer_registry.clone(),
            },
        )
        .map(|i| i.into_value())
    }
}
