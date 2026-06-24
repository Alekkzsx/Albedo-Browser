use super::*;
use super::runtime::JsRuntime;
use crate::ace::runtime::bindings::webapi::indexeddb::IDBDatabase;
use rquickjs::{Class, Ctx, Value};
use std::collections::HashMap;




pub(crate) fn dispatch_sync_events(rt: &JsRuntime) -> bool {
    let mut executed = false;
    let (is_online, background_sync_tasks, periodic_sync_tasks) = {
        let mut el = rt.event_loop.lock().unwrap_or_else(|e| e.into_inner());
        let is_online = el.is_online().unwrap_or(false);
        let bg_tasks = if is_online { el.take_pending_background_sync() } else { Vec::new() };
        let per_tasks = el.take_pending_periodic_sync();
        (is_online, bg_tasks, per_tasks)
    };

    if !background_sync_tasks.is_empty() {
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                for task in background_sync_tasks {
                    if let Ok(sync_event_obj) = rquickjs::Object::new(ctx.clone()) {
                        let _ = sync_event_obj.set("tag", task.tag.clone());
                        let _ = sync_event_obj.set("lastChance", false);
                        let script = "if (globalThis.onsync) globalThis.dispatchEvent(new Event('sync'))";
                        let _ = ctx.eval::<(), _>(script);
                        executed = true;
                    }
                }
            })
        });
    }

    if !periodic_sync_tasks.is_empty() {
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                for task in periodic_sync_tasks {
                    if let Ok(periodic_event_obj) = rquickjs::Object::new(ctx.clone()) {
                        let _ = periodic_event_obj.set("tag", task.tag.clone());
                        let _ = periodic_event_obj.set("minInterval", task.min_interval_ms);
                        let script = "if (globalThis.onperiodicsync) globalThis.dispatchEvent(new Event('periodicsync'))";
                        let _ = ctx.eval::<(), _>(script);
                        executed = true;
                    }
                }
            })
        });
    }
    executed
}
