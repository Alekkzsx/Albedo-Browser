use super::*;
use super::runtime::JsRuntime;
use crate::ace::runtime::bindings::webapi::indexeddb::IDBDatabase;
use rquickjs::{Class, Ctx, Value};
use std::collections::HashMap;




pub(crate) fn run_idle_callbacks(rt: &JsRuntime) -> bool {
    let mut executed = false;
    let frame_deadline = std::time::Instant::now() + std::time::Duration::from_millis(4);
    let idle_tasks = {
        let mut el = rt.event_loop.lock().unwrap_or_else(|e| e.into_inner());
        el.take_idle_callbacks(frame_deadline)
    };

    if !idle_tasks.is_empty() {
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                for task in idle_tasks {
                    let now = std::time::Instant::now();
                    let time_remaining_ms = if frame_deadline > now {
                        frame_deadline.duration_since(now).as_secs_f64() * 1000.0
                    } else {
                        0.0
                    };
                    let did_timeout = task.timeout_deadline.map(|d| now >= d).unwrap_or(false);

                    let script = format!(
                        "(function(cb) {{ 
                            var deadline = {{ 
                                timeRemaining: function() {{ return {:.3}; }}, 
                                didTimeout: {} 
                            }};
                            cb(deadline);
                        }})",
                        time_remaining_ms.max(0.0),
                        did_timeout
                    );

                    if let Ok(wrapper_fn) = ctx.eval::<rquickjs::Function, _>(script) {
                        if let Ok(cb) = task.callback.0.restore(&ctx) {
                            let _: rquickjs::Result<Value> = wrapper_fn.call((cb,));
                            executed = true;
                        }
                    }
                }
                while ctx.execute_pending_job() {
                    executed = true;
                }
            })
        });
    }
    executed
}
