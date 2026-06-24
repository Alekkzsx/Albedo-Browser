use super::*;
use super::runtime::JsRuntime;
use crate::ace::runtime::bindings::webapi::indexeddb::IDBDatabase;
use rquickjs::{Class, Ctx, Value};
use std::collections::HashMap;




pub(crate) fn dispatch_dom_mutations(rt: &JsRuntime) -> bool {
    let mut executed = false;
    let mut mutated_flag = rt.mutations.lock().unwrap_or_else(|e| e.into_inner());
    if *mutated_flag {
        if let Some(dom_arc) = rt.dom.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            let mut dom = dom_arc.lock().unwrap_or_else(|e| e.into_inner());
            let pending = dom.take_pending_mutations();
            if !pending.is_empty() {
                rt.with_context(|ctx| {
                    ctx.with(|ctx| {
                        let registry = rt.observer_registry.lock().unwrap_or_else(|e| e.into_inner());
                        for (callback_id, records) in pending {
                            if let Some(cb_persistent) = registry.get(&callback_id) {
                                if let Ok(callback) = cb_persistent.clone().restore(&ctx) {
                                    let arr = rquickjs::Array::new(ctx.clone()).expect("Albedo Engine: internal invariant violated");
                                    for (i, rec) in records.into_iter().enumerate() {
                                        let obj = rquickjs::Object::new(ctx.clone()).expect("Albedo Engine: internal invariant violated");
                                        let _ = obj.set("type", rec.type_.as_str());
                                        let _ = obj.set("attributeName", rec.attribute_name);
                                        let _ = obj.set("oldValue", rec.old_value);

                                        let target = wrap_element(rt, rec.target, &ctx);
                                        let _ = obj.set("target", target);

                                        let added_arr = rquickjs::Array::new(ctx.clone()).expect("Albedo Engine: internal invariant violated");
                                        for (idx, &node_idx) in rec.added_nodes.iter().enumerate() {
                                            let node = wrap_element(rt, node_idx, &ctx);
                                            let _ = added_arr.set(idx, node);
                                        }
                                        let _ = obj.set("addedNodes", added_arr);

                                        let removed_arr = rquickjs::Array::new(ctx.clone()).expect("Albedo Engine: internal invariant violated");
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
