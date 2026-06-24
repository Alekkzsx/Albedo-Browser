use super::*;
use super::runtime::JsRuntime;
use crate::ace::runtime::bindings::webapi::indexeddb::IDBDatabase;
use rquickjs::{Class, Ctx, Value};
use std::collections::HashMap;




pub(crate) fn dispatch_idb_events(rt: &JsRuntime) -> bool {
    let mut executed = false;
    let idb_events = {
        let mut el = rt.event_loop.lock().unwrap_or_else(|e| e.into_inner());
        el.receive_idb_events()
    };

    if !idb_events.is_empty() {
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                let mut registry = rt.observer_registry.lock().unwrap_or_else(|e| e.into_inner());
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
                                        worker_tx: rt.idb_worker.lock().unwrap_or_else(|e| e.into_inner()).tx.clone(),
                                        observer_registry: rt.observer_registry.clone(),
                                    };
                                    if let Ok(db_instance) = Class::instance(ctx.clone(), db) {
                                        let evt = rquickjs::Object::new(ctx.clone()).expect("Albedo Engine: internal invariant violated");
                                        let target = rquickjs::Object::new(ctx.clone()).expect("Albedo Engine: internal invariant violated");
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
                                    let err_obj = rquickjs::Object::new(ctx.clone()).expect("Albedo Engine: internal invariant violated");
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
                                        worker_tx: rt.idb_worker.lock().unwrap_or_else(|e| e.into_inner()).tx.clone(),
                                        observer_registry: rt.observer_registry.clone(),
                                    };
                                    if let Ok(db_instance) = Class::instance(ctx.clone(), db) {
                                        let upgrade_evt = rquickjs::Object::new(ctx.clone()).expect("Albedo Engine: internal invariant violated");
                                        let _ = upgrade_evt.set("target", {
                                            let target = rquickjs::Object::new(ctx.clone()).expect("Albedo Engine: internal invariant violated");
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
