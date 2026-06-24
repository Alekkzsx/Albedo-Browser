use super::*;
use super::runtime::JsRuntime;
use crate::ace::runtime::bindings::webapi::indexeddb::IDBDatabase;
use rquickjs::{Class, Ctx, Value};
use std::collections::HashMap;




pub(crate) fn flush_microtasks(rt: &JsRuntime) -> bool {
    let mut executed = false;
    let ctx = rt.context.lock().unwrap_or_else(|e| e.into_inner());
    ctx.with(|ctx| {
        while ctx.execute_pending_job() {
            tracing::debug!("Microtask/Promise executed");
            executed = true;
        }
    });
    executed
}
