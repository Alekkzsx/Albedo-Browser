use super::*;
use super::runtime::JsRuntime;
use crate::ace::runtime::bindings::webapi::indexeddb::IDBDatabase;
use rquickjs::{Class, Ctx, Value};
use std::collections::HashMap;




pub(crate) fn run_timer_tasks(rt: &JsRuntime) -> bool {
    let mut executed = false;
    let (timers, macros) = {
        let mut el = rt.event_loop.lock().unwrap_or_else(|e| e.into_inner());
        el.take_pending_tasks()
    };

    if !timers.is_empty() {
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                for timer in timers {
                    if let Ok(func) = timer.callback.0.clone().restore(&ctx) {
                        let _: rquickjs::Result<Value> = func.call(());
                        executed = true;
                    }
                }
                while ctx.execute_pending_job() {
                    executed = true;
                }
            })
        });
    }

    for task in macros {
        task();
        executed = true;
    }
    executed
}
