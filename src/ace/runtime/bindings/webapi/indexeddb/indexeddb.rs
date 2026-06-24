use super::*;
use crate::ace::runtime::bindings::webapi::idb_service::worker::IDBWorkerCommand;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};



#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IndexedDB {
    #[qjs(skip_trace)]
    origin: String,
    #[qjs(skip_trace)]
    worker_tx: tokio::sync::mpsc::UnboundedSender<IDBWorkerCommand>,
    #[qjs(skip_trace)]
    event_loop: Arc<Mutex<crate::ace::runtime::core::event_loop::EventLoop>>,
    #[qjs(skip_trace)]
    observer_registry: Arc<Mutex<HashMap<usize, Persistent<Function<'static>>>>>,
}

impl IndexedDB {
    /// TODO: add docs
    pub fn new(rt: &JsRuntime) -> Self {
        Self {
            origin: rt
                .origin
                .lock().unwrap_or_else(|e| e.into_inner())
                .as_ref()
                .map(|o| o.to_string())
                .unwrap_or_else(|| "null".into()),
            worker_tx: rt.idb_worker.lock().unwrap_or_else(|e| e.into_inner()).tx.clone(),
            event_loop: rt.event_loop.clone(),
            observer_registry: rt.observer_registry.clone(),
        }
    }
}

#[rquickjs::methods]
impl IndexedDB {
    /// TODO: add docs
    pub fn open<'js>(
        &self,
        ctx: Ctx<'js>,
        name: String,
        version: rquickjs::prelude::Opt<u32>,
    ) -> Result<Value<'js>> {
        let cb_id = self.event_loop.lock().unwrap_or_else(|e| e.into_inner()).get_next_idb_callback_id();
        let req_base = IDBRequest {
            callback_id: cb_id,
            observer_registry: self.observer_registry.clone(),
        };
        let req = IDBOpenDBRequest { request: req_base };

        let path = crate::utils::paths::config_dir()
            .join("albedo")
            .join("indexeddb")
            .join(format!(
                "idb_{}_{}.sqlite",
                self.origin.replace(":", "_"),
                name
            ));

        let _ = self.worker_tx.send(IDBWorkerCommand::OpenDb {
            request_callback_id: cb_id,
            name,
            version: version.0,
            origin: self.origin.clone(),
            db_path: path,
        });

        Class::instance(ctx, req).map(|i| i.into_value())
    }
}
