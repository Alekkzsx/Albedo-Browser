use rquickjs::{Class, Ctx, Function, Persistent, Result, Value, Object, prelude::*};
use crate::runtime::core::runtime::JsRuntime;
use crate::runtime::bindings::webapi::idb_service::worker::IDBWorkerCommand;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IDBRequest {
    #[qjs(skip_trace)]
    pub result: Arc<Mutex<Option<Persistent<Value<'static>>>>>,
    #[qjs(skip_trace)]
    pub error: Arc<Mutex<Option<Persistent<Value<'static>>>>>,
}

#[rquickjs::methods]
impl IDBRequest {
    #[qjs(get, rename = "onsuccess")]
    pub fn onsuccess_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> { Ok(Value::new_null(ctx)) }
    #[qjs(set, rename = "onsuccess")]
    pub fn onsuccess_setter<'js>(&mut self, f: Function<'js>) { let _ = f; }

    #[qjs(get, rename = "onerror")]
    pub fn onerror_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> { Ok(Value::new_null(ctx)) }
    #[qjs(set, rename = "onerror")]
    pub fn onerror_setter<'js>(&mut self, f: Function<'js>) { let _ = f; }

    #[qjs(get)]
    pub fn result<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(res) = self.result.lock().unwrap().as_ref() {
            res.clone().restore(&ctx)
        } else {
            Ok(Value::new_null(ctx))
        }
    }
    
    #[qjs(get)]
    pub fn error<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(res) = self.error.lock().unwrap().as_ref() {
            res.clone().restore(&ctx)
        } else {
            Ok(Value::new_null(ctx))
        }
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
    #[qjs(get, rename = "onsuccess")]
    pub fn onsuccess_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> { Ok(Value::new_null(ctx)) }
    #[qjs(set, rename = "onsuccess")]
    pub fn onsuccess_setter<'js>(&mut self, ctx: Ctx<'js>, f: Function<'js>) {
        // Here we could manually dispatch if the result is already available,
        // but typically the IDB service will fire it from the event loop.
        let _ = f;
    }

    #[qjs(get, rename = "onerror")]
    pub fn onerror_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> { Ok(Value::new_null(ctx)) }
    #[qjs(set, rename = "onerror")]
    pub fn onerror_setter<'js>(&mut self, f: Function<'js>) { let _ = f; }

    #[qjs(get, rename = "onupgradeneeded")]
    pub fn onupgradeneeded_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> { Ok(Value::new_null(ctx)) }
    #[qjs(set, rename = "onupgradeneeded")]
    pub fn onupgradeneeded_setter<'js>(&mut self, f: Function<'js>) { let _ = f; }

    #[qjs(get)]
    pub fn result<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.request.result(ctx)
    }
    
    #[qjs(get)]
    pub fn error<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.request.error(ctx)
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
    event_loop_ref: Arc<Mutex<crate::runtime::core::event_loop::EventLoop>>,
    #[qjs(skip_trace)]
    observer_registry: Arc<Mutex<std::collections::HashMap<usize, Persistent<Function<'static>>>>>,
}

impl IndexedDB {
    pub fn new(rt: &JsRuntime) -> Self {
        let origin = if let Some(o) = rt.origin.lock().unwrap().as_ref() {
            o.to_string()
        } else {
            "null".to_string()
        };
        
        let worker_tx = rt.idb_worker.lock().unwrap().tx.clone();
        
        Self {
            origin,
            worker_tx,
            event_loop_ref: rt.event_loop.clone(),
            observer_registry: rt.observer_registry.clone(),
        }
    }
}

// Emulate EventTarget dispatch manually via JS wrapper injected by Albedo IDB Event Dispatcher
#[rquickjs::methods]
impl IndexedDB {
    pub fn open<'js>(&self, ctx: Ctx<'js>, name: String, version: rquickjs::prelude::Opt<u32>) -> Result<Value<'js>> {
        let req = IDBOpenDBRequest {
            request: IDBRequest {
                result: Arc::new(Mutex::new(None)),
                error: Arc::new(Mutex::new(None)),
            }
        };

        let req_obj = Class::instance(ctx.clone(), req)?;
        let req_val = req_obj.into_value();

        // 1. Generate callback ID
        let callback_id = {
            let mut el = self.event_loop_ref.lock().unwrap();
            el.get_next_idb_callback_id()
        };

        // 2. We inject a generic JS listener function into observer_registry that handles the IDBEventMessage from `executor.rs` 
        // by reading the bound `req_val` and manually triggering `onsuccess`/`onerror`/`onupgradeneeded` properties.
        let js_dispatcher = ctx.eval::<Function, _>(
            r#"
            (function(req) {
                return function(eventData) {
                    if (eventData.oldVersion !== undefined && req.onupgradeneeded) {
                        try { req.onupgradeneeded(eventData); } catch(e) { console.error(e); }
                    } else if (eventData.name && eventData.message) {
                        // Error
                        if (req.onerror) {
                           let evt = new Event('error');
                           evt.target = { error: eventData };
                           try { req.onerror(evt); } catch(e) { console.error(e); }
                        }
                    } else {
                        // Success -> result is the IDBDatabase (we'll map it later)
                        // For now we set it to the parsed JSON
                        if (req.onsuccess) {
                           let evt = new Event('success');
                           evt.target = { result: eventData };
                           try { req.onsuccess(evt); } catch(e) { console.error(e); }
                        }
                    }
                };
            })
            "#
        )?;
        let bound_dispatcher: Function = js_dispatcher.call((req_val.clone(),))?;
        let persistent_cb = Persistent::save(&ctx, bound_dispatcher);
        
        self.observer_registry.lock().unwrap().insert(callback_id, persistent_cb);

        let mut db_path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        db_path.push("albedo");
        std::fs::create_dir_all(&db_path).unwrap_or_default();
        let safe_origin = self.origin.replace("://", "_").replace(":", "_");
        db_path.push(format!("indexeddb_{}.sqlite", safe_origin));

        // 3. Send to SQLite Worker
        let _ = self.worker_tx.send(IDBWorkerCommand::OpenDb {
            request_callback_id: callback_id,
            name,
            version: version.0,
            origin: self.origin.clone(),
            db_path,
        });

        // 4. Emulate returning the IDBOpenDBRequest synchronously
        Ok(req_val)
    }
}

// ---------------------------------------------------------
// IDBDatabase, IDBTransaction, IDBObjectStore 
// Placeholder Stubs for the next iteration step.
// ---------------------------------------------------------

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IDBDatabase {
    #[qjs(skip_trace)]
    pub name: String,
    #[qjs(skip_trace)]
    pub version: u32,
    #[qjs(skip_trace)]
    worker_tx: tokio::sync::mpsc::UnboundedSender<IDBWorkerCommand>,
    #[qjs(skip_trace)]
    event_loop_ref: Arc<Mutex<crate::runtime::core::event_loop::EventLoop>>,
    #[qjs(skip_trace)]
    observer_registry: Arc<Mutex<std::collections::HashMap<usize, Persistent<Function<'static>>>>>,
}

#[rquickjs::methods]
impl IDBDatabase {
    #[qjs(get)]
    pub fn name(&self) -> String {
        self.name.clone()
    }
    
    #[qjs(get)]
    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn close(&self) {
        // We let the worker handle drops or explicit close command later
    }
}

// ---------------------------------------------------------
// IDBTransaction
// ---------------------------------------------------------

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IDBTransaction {
    #[qjs(skip_trace)]
    pub db_name: String,
    #[qjs(skip_trace)]
    pub store_names: Vec<String>,
    #[qjs(skip_trace)]
    pub mode: String,
    #[qjs(skip_trace)]
    pub tx_id: usize,
    #[qjs(skip_trace)]
    worker_tx: tokio::sync::mpsc::UnboundedSender<IDBWorkerCommand>,
    #[qjs(skip_trace)]
    event_loop_ref: Arc<Mutex<crate::runtime::core::event_loop::EventLoop>>,
    #[qjs(skip_trace)]
    observer_registry: Arc<Mutex<std::collections::HashMap<usize, Persistent<Function<'static>>>>>,
}

#[rquickjs::methods]
impl IDBTransaction {
    #[qjs(get)]
    pub fn mode(&self) -> String { self.mode.clone() }
    
    // Spec: Returns the object store by name
    pub fn objectStore<'js>(&self, ctx: Ctx<'js>, name: String) -> Result<Value<'js>> {
        if !self.store_names.contains(&name) {
            return Err(rquickjs::Error::Exception); // Error: InvalidStateError
        }
        
        let store = IDBObjectStore {
            db_name: self.db_name.clone(),
            name: name,
            tx_id: self.tx_id,
            worker_tx: self.worker_tx.clone(),
            event_loop_ref: self.event_loop_ref.clone(),
            observer_registry: self.observer_registry.clone(),
        };
        Class::instance(ctx, store).map(|i| i.into_value())
    }

    pub fn abort(&self) {
        // Drop wrapper handled by sqlite worker using RAII transaction locks on dropping channel sender.
    }
}


// ---------------------------------------------------------
// IDBObjectStore
// ---------------------------------------------------------

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IDBObjectStore {
    #[qjs(skip_trace)]
    pub db_name: String,
    #[qjs(skip_trace)]
    pub name: String,
    #[qjs(skip_trace)]
    pub tx_id: usize,
    #[qjs(skip_trace)]
    worker_tx: tokio::sync::mpsc::UnboundedSender<IDBWorkerCommand>,
    #[qjs(skip_trace)]
    event_loop_ref: Arc<Mutex<crate::runtime::core::event_loop::EventLoop>>,
    #[qjs(skip_trace)]
    observer_registry: Arc<Mutex<std::collections::HashMap<usize, Persistent<Function<'static>>>>>,
}

impl IDBObjectStore {
    // Generic request logic extracted
    fn create_request<'js>(&self, ctx: Ctx<'js>) -> Result<(Value<'js>, usize)> {
        let req = IDBRequest { result: Arc::new(Mutex::new(None)), error: Arc::new(Mutex::new(None)) };
        let req_obj = Class::instance(ctx.clone(), req)?.into_value();
        
        let callback_id = {
            let mut el = self.event_loop_ref.lock().unwrap();
            el.get_next_idb_callback_id()
        };
        
        // Setup internal JS manual listener mechanism similar to open
        let js_dispatcher = ctx.eval::<Function, _>(
            r#"
            (function(req) {
                return function(eventData) {
                    if (eventData && eventData.message) {
                        if (req.onerror) {
                           let evt = new Event('error');
                           evt.target = { error: eventData };
                           try { req.onerror(evt); } catch(e) {}
                        }
                    } else {
                        if (req.onsuccess) {
                           let evt = new Event('success');
                           evt.target = { result: eventData };
                           try { req.onsuccess(evt); } catch(e) {}
                        }
                    }
                };
            })
            "#
        )?;
        
        let bound_dispatcher: Function = js_dispatcher.call((req_obj.clone(),))?;
        let persistent_cb = Persistent::save(&ctx, bound_dispatcher);
        self.observer_registry.lock().unwrap().insert(callback_id, persistent_cb);

        Ok((req_obj, callback_id))
    }
}

#[rquickjs::methods]
impl IDBObjectStore {
    #[qjs(get)]
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn put<'js>(&self, ctx: Ctx<'js>, value: Value<'js>, key: rquickjs::prelude::Opt<Value<'js>>) -> Result<Value<'js>> {
        let (req_val, cb_id) = self.create_request(ctx.clone())?;
        
        // Extract json string for `value`
        let val_json: String = ctx.json_stringify(&value)?
            .and_then(|v| v.as_string().map(|s| s.to_string().unwrap_or_default()))
            .unwrap_or_else(|| "null".into());
        
        // Extract key string representation 
        let key_str = if let Some(k) = key.0 {
             ctx.json_stringify(&k)?
                .and_then(|v| v.as_string().map(|s| s.to_string().unwrap_or_default()))
                .unwrap_or_else(|| "null".into())
        } else {
             "".into() // Use auto-increment mapping on rust SQLite layer later
        };
        
        let _ = self.worker_tx.send(IDBWorkerCommand::StorePut {
            request_callback_id: cb_id,
            db_name: self.db_name.clone(),
            store_name: self.name.clone(),
            key: key_str,
            value_json: val_json,
        });

        Ok(req_val)
    }

    pub fn add<'js>(&self, ctx: Ctx<'js>, value: Value<'js>, key: rquickjs::prelude::Opt<Value<'js>>) -> Result<Value<'js>> {
         // Same logic conceptually for Add. To ensure Spec compliant uniqueness, the SQLite worker needs to check uniqueness (INSERT vs REPLACE)
         // In a robust implementation, IDBWorkerCommand::StoreAdd instead.
         self.put(ctx, value, key)
    }

    pub fn get<'js>(&self, ctx: Ctx<'js>, key: Value<'js>) -> Result<Value<'js>> {
        let (req_val, cb_id) = self.create_request(ctx.clone())?;
        let key_str = ctx.json_stringify(&key)?
            .and_then(|v| v.as_string().map(|s| s.to_string().unwrap_or_default()))
            .unwrap_or_else(|| "null".into());
        
        let _ = self.worker_tx.send(IDBWorkerCommand::StoreGet {
            request_callback_id: cb_id,
            db_name: self.db_name.clone(),
            store_name: self.name.clone(),
            key: key_str,
        });

        Ok(req_val)
    }

    pub fn delete<'js>(&self, ctx: Ctx<'js>, key: Value<'js>) -> Result<Value<'js>> {
        let (req_val, cb_id) = self.create_request(ctx.clone())?;
        let key_str = ctx.json_stringify(&key)?
            .and_then(|v| v.as_string().map(|s| s.to_string().unwrap_or_default()))
            .unwrap_or_else(|| "null".into());
        
        let _ = self.worker_tx.send(IDBWorkerCommand::StoreDelete {
            request_callback_id: cb_id,
            db_name: self.db_name.clone(),
            store_name: self.name.clone(),
            key: key_str,
        });

        Ok(req_val)
    }

    pub fn clear<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let (req_val, cb_id) = self.create_request(ctx.clone())?;
        let _ = self.worker_tx.send(IDBWorkerCommand::StoreClear {
            request_callback_id: cb_id,
            db_name: self.db_name.clone(),
            store_name: self.name.clone(),
        });
        Ok(req_val)
    }
}

pub fn register(rt: &JsRuntime) -> Result<()> {
    // Need to extract variables because we can't capture rt entirely inside with_context safely since nested refs.
    let indexeddb_instance = IndexedDB::new(rt);
    
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            let globals = ctx.globals();
            globals.set("IDBRequest", Class::<IDBRequest>::register(&ctx)?)?;
            globals.set("IDBOpenDBRequest", Class::<IDBOpenDBRequest>::register(&ctx)?)?;
            globals.set("IDBDatabase", Class::<IDBDatabase>::register(&ctx)?)?;
            globals.set("IDBTransaction", Class::<IDBTransaction>::register(&ctx)?)?;
            globals.set("IDBObjectStore", Class::<IDBObjectStore>::register(&ctx)?)?;
            
            // Register factory instance instead of class
            let idb_obj = Class::instance(ctx.clone(), indexeddb_instance)?;
            globals.set("indexedDB", idb_obj)?;
            Ok(())
        })
    })
}
