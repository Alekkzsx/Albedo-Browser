use crate::runtime::bindings::webapi::idb_service::worker::IDBWorkerCommand;
use crate::runtime::core::runtime::JsRuntime;
use rquickjs::{prelude::*, Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

static NEXT_REQ_ID: AtomicUsize = AtomicUsize::new(1000);

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IDBRequest {
    #[qjs(skip_trace)]
    pub callback_id: usize,
    #[qjs(skip_trace)]
    pub observer_registry: Arc<Mutex<HashMap<usize, Persistent<Function<'static>>>>>,
}

#[rquickjs::methods]
impl IDBRequest {
    #[qjs(get, rename = "onsuccess")]
    pub fn get_onsuccess<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onsuccess")]
    pub fn set_onsuccess<'js>(&mut self, ctx: Ctx<'js>, f: Function<'js>) {
        let p = Persistent::save(&ctx, f);
        self.observer_registry
            .lock()
            .unwrap()
            .insert(self.callback_id, p);
    }

    #[qjs(get, rename = "onerror")]
    pub fn get_onerror<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onerror")]
    pub fn set_onerror<'js>(&mut self, ctx: Ctx<'js>, f: Function<'js>) {
        let p = Persistent::save(&ctx, f);
        self.observer_registry
            .lock()
            .unwrap()
            .insert(self.callback_id, p);
    }
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IDBOpenDBRequest {
    #[qjs(skip_trace)]
    pub request: IDBRequest,
}

#[rquickjs::methods]
impl IDBOpenDBRequest {
    #[qjs(get, rename = "onupgradeneeded")]
    pub fn get_onupgradeneeded<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onupgradeneeded")]
    pub fn set_onupgradeneeded<'js>(&mut self, ctx: Ctx<'js>, f: Function<'js>) {
        let p = Persistent::save(&ctx, f);
        self.request
            .observer_registry
            .lock()
            .unwrap()
            .insert(self.request.callback_id, p);
    }

    #[qjs(get, rename = "onsuccess")]
    pub fn get_onsuccess<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onsuccess")]
    pub fn set_onsuccess<'js>(&mut self, ctx: Ctx<'js>, f: Function<'js>) {
        let p = Persistent::save(&ctx, f);
        self.request
            .observer_registry
            .lock()
            .unwrap()
            .insert(self.request.callback_id, p);
    }
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IndexedDB {
    #[qjs(skip_trace)]
    origin: String,
    #[qjs(skip_trace)]
    worker_tx: tokio::sync::mpsc::UnboundedSender<IDBWorkerCommand>,
    #[qjs(skip_trace)]
    event_loop: Arc<Mutex<crate::runtime::core::event_loop::EventLoop>>,
    #[qjs(skip_trace)]
    observer_registry: Arc<Mutex<HashMap<usize, Persistent<Function<'static>>>>>,
}

impl IndexedDB {
    pub fn new(rt: &JsRuntime) -> Self {
        Self {
            origin: rt
                .origin
                .lock()
                .unwrap()
                .as_ref()
                .map(|o| o.to_string())
                .unwrap_or_else(|| "null".into()),
            worker_tx: rt.idb_worker.lock().unwrap().tx.clone(),
            event_loop: rt.event_loop.clone(),
            observer_registry: rt.observer_registry.clone(),
        }
    }
}

#[rquickjs::methods]
impl IndexedDB {
    pub fn open<'js>(
        &self,
        ctx: Ctx<'js>,
        name: String,
        version: rquickjs::prelude::Opt<u32>,
    ) -> Result<Value<'js>> {
        let cb_id = self.event_loop.lock().unwrap().get_next_idb_callback_id();
        let req_base = IDBRequest {
            callback_id: cb_id,
            observer_registry: self.observer_registry.clone(),
        };
        let req = IDBOpenDBRequest { request: req_base };

        let path = dirs::config_dir()
            .unwrap_or_default()
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

    pub fn createObjectStore<'js>(
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

    pub fn close(&self) {}
}

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
    pub fn objectStore<'js>(&self, ctx: Ctx<'js>, name: String) -> Result<Value<'js>> {
        let store = IDBObjectStore {
            name,
            transaction_id: self.tx_id,
            worker_tx: self.worker_tx.clone(),
            observer_registry: self.observer_registry.clone(),
        };
        Class::instance(ctx, store).map(|i| i.into_value())
    }
    pub fn commit(&self) {
        let _ = self.worker_tx.send(IDBWorkerCommand::TransactionCommit {
            transaction_id: self.tx_id,
        });
    }
    pub fn abort(&self) {
        let _ = self.worker_tx.send(IDBWorkerCommand::TransactionAbort {
            transaction_id: self.tx_id,
        });
    }
}

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

    pub fn createIndex<'js>(
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
        let idx = IDBIndex {
            name,
            store_name: self.name.clone(),
            transaction_id: self.transaction_id,
            worker_tx: self.worker_tx.clone(),
            observer_registry: self.observer_registry.clone(),
        };
        Class::instance(ctx, idx).map(|i| i.into_value())
    }

    pub fn openCursor<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
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

pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            let g = ctx.globals();
            g.set("IDBRequest", Class::<IDBRequest>::register(&ctx)?)?;
            g.set(
                "IDBOpenDBRequest",
                Class::<IDBOpenDBRequest>::register(&ctx)?,
            )?;
            g.set("IDBDatabase", Class::<IDBDatabase>::register(&ctx)?)?;
            g.set("IDBTransaction", Class::<IDBTransaction>::register(&ctx)?)?;
            g.set("IDBObjectStore", Class::<IDBObjectStore>::register(&ctx)?)?;
            g.set("IDBIndex", Class::<IDBIndex>::register(&ctx)?)?;
            g.set(
                "indexedDB",
                Class::instance(ctx.clone(), IndexedDB::new(rt))?,
            )?;
            Ok(())
        })
    })
}
