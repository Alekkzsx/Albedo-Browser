use super::*;
use crate::ace::runtime::bindings::webapi::idb_service::worker::IDBWorkerCommand;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};



#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IDBObjectStore {
    pub name: String,
    pub transaction_id: usize,
    #[qjs(skip_trace)]
    worker_tx: tokio::sync::mpsc::UnboundedSender<IDBWorkerCommand>,
    #[qjs(skip_trace)]
    observer_registry: Arc<Mutex<HashMap<usize, Persistent<Function<'static>>>>>,
}

#[rquickjs::methods]
impl IDBObjectStore {
    /// TODO: add docs
    pub fn put<'js>(
        &self,
        ctx: Ctx<'js>,
        val: Value<'js>,
        key: rquickjs::prelude::Opt<Value<'js>>,
    ) -> Result<Value<'js>> {
        let cb_id = NEXT_REQ_ID.fetch_add(1, Ordering::SeqCst);
        let val_json: String = ctx
            .json_stringify(&val)?
            .and_then(|v| v.as_string().and_then(|s| s.to_string().ok()))
            .unwrap_or_default();
        let key_str = if let Some(k) = key.0 {
            k.as_string()
                .and_then(|s| s.to_string().ok())
                .unwrap_or_default()
        } else {
            "".into()
        };
        let _ = self.worker_tx.send(IDBWorkerCommand::StorePut {
            request_callback_id: cb_id,
            transaction_id: self.transaction_id,
            store_name: self.name.clone(),
            key: key_str,
            value_json: val_json,
            overwrite: true,
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

    /// TODO: add docs
    pub fn get<'js>(&self, ctx: Ctx<'js>, key: String) -> Result<Value<'js>> {
        let cb_id = NEXT_REQ_ID.fetch_add(1, Ordering::SeqCst);
        let _ = self.worker_tx.send(IDBWorkerCommand::StoreGet {
            request_callback_id: cb_id,
            transaction_id: self.transaction_id,
            store_name: self.name.clone(),
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

    #[qjs(rename = "createIndex")]
    pub fn create_index<'js>(
        &self,
        ctx: Ctx<'js>,
        name: String,
        key_path: String,
        opt: rquickjs::prelude::Opt<Object<'js>>,
    ) -> Result<Value<'js>> {
        let unique = opt
            .0
            .and_then(|o| o.get::<_, bool>("unique").ok())
            .unwrap_or(false);
        let _ = self.worker_tx.send(IDBWorkerCommand::CreateIndex {
            transaction_id: self.transaction_id,
            store_name: self.name.clone(),
            index_name: name.clone(),
            key_path,
            unique,
        });
        let index = IDBIndex {
            name,
            store_name: self.name.clone(),
            transaction_id: self.transaction_id,
            worker_tx: self.worker_tx.clone(),
            observer_registry: self.observer_registry.clone(),
        };
        Class::instance(ctx, index).map(|i| i.into_value())
    }

    #[qjs(rename = "openCursor")]
    pub fn open_cursor<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let cb_id = NEXT_REQ_ID.fetch_add(1, Ordering::SeqCst);
        let _ = self.worker_tx.send(IDBWorkerCommand::OpenCursor {
            request_callback_id: cb_id,
            transaction_id: self.transaction_id,
            store_name: self.name.clone(),
            index_name: None,
            range: None,
            direction: "next".into(),
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
