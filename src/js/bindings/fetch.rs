use rquickjs::{Ctx, Result, Persistent, Promise, Class, Value};
use crate::js::JsRuntime;

#[derive(rquickjs::class::Trace, Clone)]
#[rquickjs::class]
pub struct Response {
    pub status: u16,
    #[qjs(skip_trace)]
    pub body: String,
}

#[rquickjs::methods]
impl Response {
    pub fn text<'js>(&self, ctx: Ctx<'js>) -> Result<Promise<'js>> {
        let (promise, resolver, _) = Promise::new(&ctx)?;
        resolver.resolve(self.body.clone())?;
        Ok(promise)
    }

    pub fn json<'js>(&self, ctx: Ctx<'js>) -> Result<Promise<'js>> {
        let (promise, resolver, _) = Promise::new(&ctx)?;
        // Use JSON.parse via globals
        let json: rquickjs::Object = ctx.globals().get("JSON")?;
        let parse: rquickjs::Function = json.get("parse")?;
        match parse.call((self.body.clone(),)) {
            Ok(val) => resolver.resolve(val)?,
            Err(e) => resolver.reject(e.to_string())?,
        }
        Ok(promise)
    }

    #[qjs(get)]
    pub fn ok(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}

pub fn register(rt: &JsRuntime) -> rquickjs::Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            let global = ctx.globals();
            
            // Define Response class
            Class::<Response>::define(&global)?;

            let rt_clone = rt.clone();
            let fetch_func = rquickjs::Function::new(ctx.clone(), move |ctx: Ctx<'_>, url: String| -> Result<Promise<'_>> {
                let (promise, resolver, _) = Promise::new(&ctx)?;
                let persistent_resolver = Persistent::save(&ctx, resolver);
                let rt_inner = rt_clone.clone();
                
                tokio::spawn(async move {
                    let result = reqwest::get(&url).await;
                    let final_result = match result {
                        Ok(resp) => {
                            let status = resp.status().as_u16();
                            let body = resp.text().await.unwrap_or_default();
                            Ok((status, body))
                        }
                        Err(e) => Err(e.to_string()),
                    };
                    
                    if let Ok(mut el) = rt_inner.event_loop.lock() {
                        el.queue_macro_task(move || {
                            rt_inner.with_context(|ctx| {
                                ctx.with(|ctx| {
                                    if let Ok(resolver) = persistent_resolver.restore(&ctx) {
                                        match final_result {
                                            Ok((status, body)) => {
                                                let response = Response { status, body };
                                                if let Ok(instance) = Class::instance(ctx.clone(), response) {
                                                    let _ = resolver.resolve(instance);
                                                }
                                            }
                                            Err(err) => {
                                                let _ = resolver.reject(err);
                                            }
                                        }
                                    }
                                });
                            });
                        });
                    }
                });
                
                Ok(promise)
            })?;
            
            global.set("fetch", fetch_func)?;
            Ok(())
        })
    })
}
