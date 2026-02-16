use rquickjs::{Class, Ctx, Function, Persistent, Result, Value, Object};
use crate::js::JsRuntime;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Default)]
struct DatabaseSchema {
    version: u32,
    stores: HashMap<String, ObjectStoreData>,
}

#[derive(Serialize, Deserialize, Default)]
struct ObjectStoreData {
    key_path: Option<String>,
    auto_increment: bool,
    data: HashMap<String, serde_json::Value>,
}

#[rquickjs::class]
#[derive(Clone, rquickjs::class::Trace)]
pub struct IDBRequest {
    pub result: Arc<Mutex<Option<Persistent<Value<'static>>>>>,
    pub onsuccess: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
    pub onerror: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
}

#[rquickjs::methods]
impl IDBRequest {
    #[qjs(get)]
    pub fn result<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(ref res) = *self.result.lock().unwrap() {
            res.restore(&ctx)
        } else {
            Ok(Value::new_null(ctx))
        }
    }

    #[qjs(get)]
    pub fn onsuccess<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(ref cb) = *self.onsuccess.lock().unwrap() {
            cb.restore(&ctx).map(|f| f.into_value())
        } else {
            Ok(Value::new_null(ctx))
        }
    }

    #[qjs(set)]
    pub fn set_onsuccess(&self, ctx: Ctx<'_>, func: Function<'_>) -> Result<()> {
        *self.onsuccess.lock().unwrap() = Some(Persistent::save(ctx, func));
        Ok(())
    }

    #[qjs(get)]
    pub fn onerror<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(ref cb) = *self.onerror.lock().unwrap() {
            cb.restore(&ctx).map(|f| f.into_value())
        } else {
            Ok(Value::new_null(ctx))
        }
    }

    #[qjs(set)]
    pub fn set_onerror(&self, ctx: Ctx<'_>, func: Function<'_>) -> Result<()> {
        *self.onerror.lock().unwrap() = Some(Persistent::save(ctx, func));
        Ok(())
    }
}

#[rquickjs::class]
#[derive(Clone, rquickjs::class::Trace)]
pub struct IDBOpenDBRequest {
    #[qjs(skip_trace)]
    pub base: IDBRequest,
    pub onupgradeneeded: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
    pub onblocked: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
}

#[rquickjs::methods]
impl IDBOpenDBRequest {
    // Manually delegate IDBRequest methods because rquickjs doesn't support inheritance directly in class macros easily
    #[qjs(get)]
    pub fn result<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> { self.base.result(ctx) }
    
    #[qjs(get)]
    pub fn onsuccess<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> { self.base.onsuccess(ctx) }
    #[qjs(set)]
    pub fn set_onsuccess(&self, ctx: Ctx<'_>, func: Function<'_>) -> Result<()> { self.base.set_onsuccess(ctx, func) }
    
    #[qjs(get)]
    pub fn onerror<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> { self.base.onerror(ctx) }
    #[qjs(set)]
    pub fn set_onerror(&self, ctx: Ctx<'_>, func: Function<'_>) -> Result<()> { self.base.set_onerror(ctx, func) }

    #[qjs(get)]
    pub fn onupgradeneeded<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(ref cb) = *self.onupgradeneeded.lock().unwrap() {
            cb.restore(&ctx).map(|f| f.into_value())
        } else {
            Ok(Value::new_null(ctx))
        }
    }

    #[qjs(set)]
    pub fn set_onupgradeneeded(&self, ctx: Ctx<'_>, func: Function<'_>) -> Result<()> {
        *self.onupgradeneeded.lock().unwrap() = Some(Persistent::save(ctx, func));
        Ok(())
    }
}

#[rquickjs::class]
#[derive(Clone, rquickjs::class::Trace)]
pub struct IDBDatabase {
    pub name: String,
    pub version: u32,
    #[qjs(skip_trace)]
    schema: Arc<Mutex<DatabaseSchema>>,
    #[qjs(skip_trace)]
    path: PathBuf,
}

#[rquickjs::methods]
impl IDBDatabase {
    pub fn close(&self) {}
    
    #[qjs(rename = "createObjectStore")]
    pub fn create_object_store(&self, name: String, options: Option<Object<'_>>) -> Result<IDBObjectStore> {
        let mut schema = self.schema.lock().unwrap();
        let key_path = options.as_ref().and_then(|o| o.get::<_, Option<String>>("keyPath").unwrap_or(None));
        let auto_increment = options.as_ref().and_then(|o| o.get::<_, bool>("autoIncrement").ok()).unwrap_or(false);
        
        schema.stores.insert(name.clone(), ObjectStoreData {
            key_path,
            auto_increment,
            data: HashMap::new(),
        });
        
        Ok(IDBObjectStore {
            name,
            database: self.clone(),
        })
    }

    pub fn transaction(&self, _store_names: Value<'_>, _mode: Option<String>) -> Result<IDBTransaction> {
        Ok(IDBTransaction {
            database: self.clone(),
        })
    }
}

#[rquickjs::class]
#[derive(Clone, rquickjs::class::Trace)]
pub struct IDBTransaction {
    pub database: IDBDatabase,
}

#[rquickjs::methods]
impl IDBTransaction {
    pub fn object_store(&self, name: String) -> Result<IDBObjectStore> {
        Ok(IDBObjectStore {
            name,
            database: self.database.clone(),
        })
    }
    
    pub fn abort(&self) {}
}

#[rquickjs::class]
#[derive(Clone, rquickjs::class::Trace)]
pub struct IDBObjectStore {
    pub name: String,
    pub database: IDBDatabase,
}

#[rquickjs::methods]
impl IDBObjectStore {
    pub fn put(&self, ctx: Ctx<'_>, value: Value<'_>, key: Option<Value<'_>>) -> Result<IDBRequest> {
        let request = IDBRequest {
            result: Arc::new(Mutex::new(None)),
            onsuccess: Arc::new(Mutex::new(None)),
            onerror: Arc::new(Mutex::new(None)),
        };

        let db = self.database.clone();
        let store_name = self.name.clone();
        let rt = ctx.userdata::<JsRuntime>().expect("JsRuntime required").clone();
        let req_clone = request.clone();
        
        // Serialize value to JSON using rquickjs JSON.stringify
        let value_json = {
            let json_obj = ctx.globals().get::<_, rquickjs::Object>("JSON")?;
            let stringify = json_obj.get::<_, rquickjs::Function>("stringify")?;
            let json_str: String = stringify.call((value.clone(),))?;
            serde_json::from_str::<serde_json::Value>(&json_str).unwrap_or(serde_json::Value::Null)
        };
        
        let key_str = key.and_then(|k| k.as_string().map(|s| s.to_string().unwrap_or_default())).unwrap_or_else(|| "default".to_string());

        tokio::spawn(async move {
            let mut schema = db.schema.lock().unwrap();
            if let Some(store) = schema.stores.get_mut(&store_name) {
                store.data.insert(key_str.clone(), value_json);
                let _ = std::fs::write(&db.path, serde_json::to_string(&*schema).unwrap());
            }

            let mut el = rt.event_loop.lock().unwrap();
            el.queue_macro_task(move || {
                rt.with_context(|ctx| {
                    ctx.with(|ctx| {
                        if let Some(ref cb) = *req_clone.onsuccess.lock().unwrap() {
                            if let Ok(func) = cb.restore(&ctx) {
                                let _ = func.call::<(), ()>(());
                            }
                        }
                    });
                });
            });
        });

        Ok(request)
    }

    pub fn get(&self, ctx: Ctx<'_>, key: Value<'_>) -> Result<IDBRequest> {
        let request = IDBRequest {
            result: Arc::new(Mutex::new(None)),
            onsuccess: Arc::new(Mutex::new(None)),
            onerror: Arc::new(Mutex::new(None)),
        };

        let db = self.database.clone();
        let store_name = self.name.clone();
        let rt = ctx.userdata::<JsRuntime>().expect("JsRuntime required").clone();
        let req_clone = request.clone();
        let key_str = key.as_string().map(|s| s.to_string().unwrap_or_default()).unwrap_or_default();

        tokio::spawn(async move {
            let schema = db.schema.lock().unwrap();
            let result_json = schema.stores.get(&store_name)
                .and_then(|s| s.data.get(&key_str))
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            let mut el = rt.event_loop.lock().unwrap();
            el.queue_macro_task(move || {
                rt.with_context(|ctx| {
                    ctx.with(|ctx| {
                        let result_val = {
                            let json_obj = ctx.globals().get::<_, rquickjs::Object>("JSON").unwrap();
                            let parse = json_obj.get::<_, rquickjs::Function>("parse").unwrap();
                            let json_str = serde_json::to_string(&result_json).unwrap();
                            parse.call::<_, Value>((json_str,)).unwrap()
                        };
                        *req_clone.result.lock().unwrap() = Some(Persistent::save(ctx, result_val));
                        
                        if let Some(ref cb) = *req_clone.onsuccess.lock().unwrap() {
                            if let Ok(func) = cb.restore(&ctx) {
                                let _ = func.call::<(), ()>(());
                            }
                        }
                    });
                });
            });
        });

        Ok(request)
    }
}

#[rquickjs::class]
pub struct IDBFactory {}

#[rquickjs::methods]
impl IDBFactory {
    pub fn open(&self, ctx: Ctx<'_>, name: String, version: Option<u32>) -> Result<IDBOpenDBRequest> {
        let request = IDBOpenDBRequest {
            base: IDBRequest {
                result: Arc::new(Mutex::new(None)),
                onsuccess: Arc::new(Mutex::new(None)),
                onerror: Arc::new(Mutex::new(None)),
            },
            onupgradeneeded: Arc::new(Mutex::new(None)),
            onblocked: Arc::new(Mutex::new(None)),
        };

        let rt = ctx.userdata::<JsRuntime>().expect("JsRuntime required").clone();
        let req_clone = request.clone();
        let version = version.unwrap_or(1);

        tokio::spawn(async move {
            let mut path = std::env::home_dir().unwrap_or_else(|| PathBuf::from("."));
            path.push(".local/share/albedo/indexeddb");
            let _ = std::fs::create_dir_all(&path);
            path.push(format!("{}.json", name));

            let schema = if path.exists() {
                std::fs::read_to_string(&path).ok()
                    .and_then(|s| serde_json::from_str::<DatabaseSchema>(&s).ok())
                    .unwrap_or_default()
            } else {
                DatabaseSchema::default()
            };

            let old_version = schema.version;
            let schema_arc = Arc::new(Mutex::new(schema));
            
            let db = IDBDatabase {
                name: name.clone(),
                version,
                schema: schema_arc.clone(),
                path: path.clone(),
            };

            let mut el = rt.event_loop.lock().unwrap();
            el.queue_macro_task(move || {
                rt.with_context(|ctx| {
                    ctx.with(|ctx| {
                        let db_instance = Class::instance(ctx.clone(), db.clone()).unwrap();
                        *req_clone.base.result.lock().unwrap() = Some(Persistent::save(ctx, db_instance.clone().into_value()));

                        if version > old_version {
                            if let Some(ref cb) = *req_clone.onupgradeneeded.lock().unwrap() {
                                if let Ok(func) = cb.restore(&ctx) {
                                    let event = rquickjs::Object::new(ctx.clone()).unwrap();
                                    let _ = event.set("oldVersion", old_version);
                                    let _ = event.set("newVersion", version);
                                    let _ = func.call::<(Object,), ()>((event,));
                                }
                            }
                            db.schema.lock().unwrap().version = version;
                        }

                        if let Some(ref cb) = *req_clone.base.onsuccess.lock().unwrap() {
                            if let Ok(func) = cb.restore(&ctx) {
                                let _ = func.call::<(), ()>(());
                            }
                        }
                    });
                });
            });
        });

        Ok(request)
    }
}

pub fn register(rt: &JsRuntime) -> rquickjs::Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            let globals = ctx.globals();
            
            Class::<IDBRequest>::register(ctx.clone())?;
            Class::<IDBOpenDBRequest>::register(ctx.clone())?;
            Class::<IDBDatabase>::register(ctx.clone())?;
            Class::<IDBObjectStore>::register(ctx.clone())?;
            Class::<IDBTransaction>::register(ctx.clone())?;
            
            let factory = Class::instance(ctx.clone(), IDBFactory {})?;
            globals.set("indexedDB", factory)?;
            
            Ok(())
        })
    })
}
