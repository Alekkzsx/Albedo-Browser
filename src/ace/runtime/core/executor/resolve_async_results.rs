use super::*;
use super::runtime::JsRuntime;
use crate::ace::runtime::bindings::webapi::indexeddb::IDBDatabase;
use rquickjs::{Class, Ctx, Value};
use std::collections::HashMap;




pub(crate) fn resolve_async_results(rt: &JsRuntime) -> bool {
    let mut executed = false;
    let async_results = {
        let mut el = rt.event_loop.lock().unwrap_or_else(|e| e.into_inner());
        el.receive_async_results()
    };

    if !async_results.is_empty() {
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                let mut el = rt.event_loop.lock().unwrap_or_else(|e| e.into_inner());
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
