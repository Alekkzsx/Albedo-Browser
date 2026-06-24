use super::runtime::JsRuntime;
use crate::ace::runtime::bindings::webapi::indexeddb::IDBDatabase;
use rquickjs::{Class, Ctx, Value};
use std::collections::HashMap;

fn dispatch_post_messages(rt: &JsRuntime) -> bool {
    let mut executed = false;
    let messages: std::collections::VecDeque<_> =
        { rt.event_loop.lock().unwrap().take_pending_messages() };
    if !messages.is_empty() {
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                for msg in messages {
                    let safe_data = msg.data_json.replace('\'', "\\'");
                    let safe_origin = msg.origin.replace('\'', "\\'");
                    let script = format!(
                        "globalThis.dispatchEvent(new MessageEvent('message', {{ data: {}, origin: '{}' }}))",
                        safe_data, safe_origin
                    );
                    let _ = ctx.eval::<(), _>(script);
                    executed = true;
                }
            })
        });
    }
    executed
}

fn dispatch_dom_mutations(rt: &JsRuntime) -> bool {
    let mut executed = false;
    let mut mutated_flag = rt.mutations.lock().unwrap();
    if *mutated_flag {
        if let Some(dom_arc) = rt.dom.lock().unwrap().as_ref() {
            let mut dom = dom_arc.lock().unwrap();
            let pending = dom.take_pending_mutations();
            if !pending.is_empty() {
                rt.with_context(|ctx| {
                    ctx.with(|ctx| {
                        let registry = rt.observer_registry.lock().unwrap();
                        for (callback_id, records) in pending {
                            if let Some(cb_persistent) = registry.get(&callback_id) {
                                if let Ok(callback) = cb_persistent.clone().restore(&ctx) {
                                    let arr = rquickjs::Array::new(ctx.clone()).unwrap();
                                    for (i, rec) in records.into_iter().enumerate() {
                                        let obj = rquickjs::Object::new(ctx.clone()).unwrap();
                                        let _ = obj.set("type", rec.type_.as_str());
                                        let _ = obj.set("attributeName", rec.attribute_name);
                                        let _ = obj.set("oldValue", rec.old_value);

                                        let target = wrap_element(rt, rec.target, &ctx);
                                        let _ = obj.set("target", target);

                                        let added_arr = rquickjs::Array::new(ctx.clone()).unwrap();
                                        for (idx, &node_idx) in rec.added_nodes.iter().enumerate() {
                                            let node = wrap_element(rt, node_idx, &ctx);
                                            let _ = added_arr.set(idx, node);
                                        }
                                        let _ = obj.set("addedNodes", added_arr);

                                        let removed_arr = rquickjs::Array::new(ctx.clone()).unwrap();
                                        for (idx, &node_idx) in rec.removed_nodes.iter().enumerate() {
                                            let node = wrap_element(rt, node_idx, &ctx);
                                            let _ = removed_arr.set(idx, node);
                                        }
                                        let _ = obj.set("removedNodes", removed_arr);

                                        if let Some(prev) = rec.previous_sibling {
                                            let _ = obj.set("previousSibling", wrap_element(rt, prev, &ctx));
                                        } else {
                                            let _ = obj.set("previousSibling", rquickjs::Value::new_null(ctx.clone()));
                                        }

                                        if let Some(next) = rec.next_sibling {
                                            let _ = obj.set("nextSibling", wrap_element(rt, next, &ctx));
                                        } else {
                                            let _ = obj.set("nextSibling", rquickjs::Value::new_null(ctx.clone()));
                                        }

                                        let _ = arr.set(i, obj);
                                    }
                                    let _: rquickjs::Result<Value> = callback.call((arr,));
                                    executed = true;
                                }
                            }
                        }
                    });
                });
            }
        }
        *mutated_flag = false;
    }
    executed
}

fn flush_microtasks(rt: &JsRuntime) -> bool {
    let mut executed = false;
    let ctx = rt.context.lock().unwrap();
    ctx.with(|ctx| {
        while ctx.execute_pending_job() {
            tracing::debug!("Microtask/Promise executed");
            executed = true;
        }
    });
    executed
}

fn resolve_async_results(rt: &JsRuntime) -> bool {
    let mut executed = false;
    let async_results = {
        let mut el = rt.event_loop.lock().unwrap();
        el.receive_async_results()
    };

    if !async_results.is_empty() {
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                let mut el = rt.event_loop.lock().unwrap();
                for res in async_results {
                    if let Some(resolution) = el.take_resolution(res.id) {
                        match res.result {
                            Ok((status, body)) => {
                                if let Ok(resolve) = resolution.resolve.0.clone().restore(&ctx) {
                                    use crate::ace::runtime::bindings::webapi::fetch::Response;
                                    let response = Response {
                                        status,
                                        body,
                                        headers: crate::ace::runtime::bindings::webapi::fetch::Headers::new(),
                                    };
                                    if let Ok(instance) = rquickjs::Class::instance(ctx.clone(), response) {
                                        let _: rquickjs::Result<()> = resolve.call((instance,));
                                        executed = true;
                                    }
                                }
                            }
                            Err(err) => {
                                if let Ok(reject) = resolution.reject.0.clone().restore(&ctx) {
                                    let _: rquickjs::Result<()> = reject.call((err,));
                                    executed = true;
                                }
                            }
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

fn dispatch_idb_events(rt: &JsRuntime) -> bool {
    let mut executed = false;
    let idb_events = {
        let mut el = rt.event_loop.lock().unwrap();
        el.receive_idb_events()
    };

    if !idb_events.is_empty() {
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                let mut registry = rt.observer_registry.lock().unwrap();
                for event in idb_events {
                    match event {
                        crate::ace::runtime::core::event_loop::IDBEventMessage::Success {
                            callback_id, result_json,
                        } => {
                            if let Some(cb_persistent) = registry.remove(&callback_id) {
                                if let Ok(callback) = cb_persistent.clone().restore(&ctx) {
                                    let val: Value = ctx
                                        .eval(format!("({})", result_json))
                                        .unwrap_or_else(|_| rquickjs::Value::new_null(ctx.clone()));
                                    let _: rquickjs::Result<Value> = callback.call((val,));
                                    executed = true;
                                }
                            }
                        }
                        crate::ace::runtime::core::event_loop::IDBEventMessage::DatabaseSuccess {
                            callback_id, db_name, version,
                        } => {
                            if let Some(cb_persistent) = registry.remove(&callback_id) {
                                if let Ok(callback) = cb_persistent.clone().restore(&ctx) {
                                    let db = IDBDatabase {
                                        name: db_name,
                                        version,
                                        worker_tx: rt.idb_worker.lock().unwrap().tx.clone(),
                                        observer_registry: rt.observer_registry.clone(),
                                    };
                                    if let Ok(db_instance) = Class::instance(ctx.clone(), db) {
                                        let evt = rquickjs::Object::new(ctx.clone()).unwrap();
                                        let target = rquickjs::Object::new(ctx.clone()).unwrap();
                                        let _ = target.set("result", db_instance);
                                        let _ = evt.set("target", target);
                                        let _: rquickjs::Result<Value> = callback.call((evt,));
                                        executed = true;
                                    }
                                }
                            }
                        }
                        crate::ace::runtime::core::event_loop::IDBEventMessage::Error {
                            callback_id, error_name, error_message,
                        } => {
                            if let Some(cb_persistent) = registry.remove(&callback_id) {
                                if let Ok(callback) = cb_persistent.clone().restore(&ctx) {
                                    let err_obj = rquickjs::Object::new(ctx.clone()).unwrap();
                                    let _ = err_obj.set("name", error_name);
                                    let _ = err_obj.set("message", error_message);
                                    let _: rquickjs::Result<Value> = callback.call((err_obj,));
                                    executed = true;
                                }
                            }
                        }
                        crate::ace::runtime::core::event_loop::IDBEventMessage::UpgradeNeeded {
                            request_callback_id, transaction_id: _, db_name, old_version, new_version,
                        } => {
                            if let Some(cb_persistent) = registry.get(&request_callback_id) {
                                if let Ok(callback) = cb_persistent.clone().restore(&ctx) {
                                    let db = IDBDatabase {
                                        name: db_name,
                                        version: new_version,
                                        worker_tx: rt.idb_worker.lock().unwrap().tx.clone(),
                                        observer_registry: rt.observer_registry.clone(),
                                    };
                                    if let Ok(db_instance) = Class::instance(ctx.clone(), db) {
                                        let upgrade_evt = rquickjs::Object::new(ctx.clone()).unwrap();
                                        let _ = upgrade_evt.set("target", {
                                            let target = rquickjs::Object::new(ctx.clone()).unwrap();
                                            let _ = target.set("result", db_instance);
                                            target
                                        });
                                        let _ = upgrade_evt.set("oldVersion", old_version);
                                        let _ = upgrade_evt.set("newVersion", new_version);
                                        let _: rquickjs::Result<Value> = callback.call((upgrade_evt,));
                                        executed = true;
                                    }
                                }
                            }
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

fn run_timer_tasks(rt: &JsRuntime) -> bool {
    let mut executed = false;
    let (timers, macros) = {
        let mut el = rt.event_loop.lock().unwrap();
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

fn run_idle_callbacks(rt: &JsRuntime) -> bool {
    let mut executed = false;
    let frame_deadline = std::time::Instant::now() + std::time::Duration::from_millis(4);
    let idle_tasks = {
        let mut el = rt.event_loop.lock().unwrap();
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

fn dispatch_sync_events(rt: &JsRuntime) -> bool {
    let mut executed = false;
    let el = rt.event_loop.lock().unwrap();

    if let Ok(is_online) = el.is_online() {
        if is_online {
            let background_sync_tasks = el.take_pending_background_sync();
            if !background_sync_tasks.is_empty() {
                drop(el);
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
        }
    }

    let periodic_sync_tasks = el.take_pending_periodic_sync();
    if !periodic_sync_tasks.is_empty() {
        drop(el);
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

fn check_stylesheet_dirty(rt: &JsRuntime) -> bool {
    if let Ok(mut sd) = rt.stylesheet_dirty.lock() {
        if *sd {
            *sd = false;
            return true;
        }
    }
    false
}

pub fn run_pending(rt: &JsRuntime) -> (bool, bool) {
    let mut executed = false;

    executed |= dispatch_post_messages(rt);
    executed |= dispatch_dom_mutations(rt);
    executed |= flush_microtasks(rt);
    executed |= resolve_async_results(rt);
    executed |= dispatch_idb_events(rt);
    executed |= run_timer_tasks(rt);
    executed |= run_idle_callbacks(rt);
    executed |= dispatch_sync_events(rt);

    let stylesheet_dirty = check_stylesheet_dirty(rt);

    check_layout_observers(rt);
    check_media_query_changes(rt);

    (executed, stylesheet_dirty)
}