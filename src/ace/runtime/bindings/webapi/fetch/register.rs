use super::*;
use crate::ace::runtime::core::event_loop::AsyncResult;
use crate::ace::runtime::core::service_worker::{
    CacheMode, InterceptResult, RedirectMode, RequestContext,
};
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;



/// TODO: add docs
pub fn register(rt: &JsRuntime) -> rquickjs::Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            let global = ctx.globals();

            global.set("Headers", Class::<Headers>::register(&ctx)?)?;
            global.set("Request", Class::<Request>::register(&ctx)?)?;
            global.set("AbortController", Class::<AbortController>::register(&ctx)?)?;
            global.set("AbortSignal", Class::<AbortSignal>::register(&ctx)?)?;

            let rt_clone = rt.clone();
            let internal_fetch = rquickjs::Function::new(
                ctx.clone(),
                move |url: String,
                      options: rquickjs::Object,
                      resolvers: rquickjs::Array|
                      -> Result<()> {
                    let ctx = resolvers.ctx();
                    let resolve: Function = resolvers.get(0)?;
                    let reject: Function = resolvers.get(1)?;

                    let (method, body, headers_map) = extract_headers_from_options(&options);

                    let (id, sender) = {
                        let mut el = rt_clone.event_loop.lock().unwrap_or_else(|e| e.into_inner());
                        let id = el.register_promise(
                            Persistent::save(&ctx, resolve),
                            Persistent::save(&ctx, reject),
                        );
                        (id, el.async_sender.clone())
                    };

                    let origin_arc = rt_clone.origin.clone();
                    let resource_manager = rt_clone.resource_manager.clone();
                    let sw_manager = rt_clone.sw_manager.clone();

                    tokio::spawn(execute_fetch(
                        url, method, body, headers_map, id, sender,
                        origin_arc, resource_manager, sw_manager,
                    ));

                    Ok(())
                },
            )?;

            global.set("__internal_fetch", internal_fetch)?;

            ctx.eval::<(), _>(
                r#"
                globalThis.fetch = function(url, options) {
                    options = options || {};
                    return new Promise((resolve, reject) => {
                        __internal_fetch(url, options, [resolve, reject]);
                    });
                };
            "#,
            )?;
            Ok(())
        })
    })
}
