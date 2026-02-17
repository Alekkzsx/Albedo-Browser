use rquickjs::{Class, Ctx, Function, Persistent, Result, Value, Object, prelude::*};
use crate::runtime::core::runtime::JsRuntime;
use std::sync::{Arc, Mutex};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IDBRequest {
    #[qjs(skip_trace)]
    pub result: Arc<Mutex<Option<Persistent<Value<'static>>>>>,
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
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IDBOpenDBRequest {
    pub request: IDBRequest,
}

#[rquickjs::methods]
impl IDBOpenDBRequest {
    #[qjs(get, rename = "onsuccess")]
    pub fn onsuccess_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> { Ok(Value::new_null(ctx)) }
    #[qjs(set, rename = "onsuccess")]
    pub fn onsuccess_setter<'js>(&mut self, f: Function<'js>) { let _ = f; }

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
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct IndexedDB {
}

#[rquickjs::methods]
impl IndexedDB {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    pub fn open<'js>(&self, ctx: Ctx<'js>, _name: String, _version: Option<u32>) -> Result<Value<'js>> {
        let req = IDBOpenDBRequest {
            request: IDBRequest {
                result: Arc::new(Mutex::new(None)),
            }
        };
        Class::instance(ctx, req).map(|i| i.into_value())
    }
}

pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            let globals = ctx.globals();
            globals.set("IDBRequest", Class::<IDBRequest>::register(&ctx)?)?;
            globals.set("IDBOpenDBRequest", Class::<IDBOpenDBRequest>::register(&ctx)?)?;
            globals.set("indexedDB", Class::<IndexedDB>::register(&ctx)?)?;
            Ok(())
        })
    })
}
