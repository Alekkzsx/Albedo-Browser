use super::*;
use crate::ace::runtime::bindings::webapi::idb_service::worker::IDBWorkerCommand;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};



/// TODO: add docs
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
