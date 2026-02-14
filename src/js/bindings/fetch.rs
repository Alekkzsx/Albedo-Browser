use rquickjs::{Ctx, Result, Persistent, Class, Function, Value};
use crate::js::JsRuntime;
use crate::js::event_loop::AsyncResult;

#[derive(rquickjs::class::Trace, Clone)]
#[rquickjs::class]
pub struct Response {
    pub status: u16,
    #[qjs(skip_trace)]
    pub body: String,
}

#[rquickjs::methods]
impl Response {
    pub fn text<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ = resolve.call::<(String,), ()>((self.body.clone(),));
        Ok(promise.into_value())
    }

    pub fn json<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let (promise, resolve, reject) = rquickjs::Promise::new(&ctx)?;
        let json: rquickjs::Object = ctx.globals().get("JSON")?;
        let parse: rquickjs::Function = json.get("parse")?;
        // Specify return type for call explicitly
        match parse.call::<(String,), Value<'js>>((self.body.clone(),)) {
            Ok(val) => { let _ = resolve.call::<(Value<'js>,), ()>((val,)); }
            Err(e) => { let _ = reject.call::<(String,), ()>((e.to_string(),)); }
        }
        Ok(promise.into_value())
    }

    #[qjs(get)]
    pub fn ok(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}

struct InternalFetch {
    rt: JsRuntime,
}

impl<'js> rquickjs::function::Func<'js> for InternalFetch {
    fn call(self, ctx: Ctx<'js>, args: rquickjs::function::Args<'js>) -> Result<()> {
        let (url, resolve, reject): (String, Function<'js>, Function<'js>) = args.into_args()?;
        
        let (id, sender) = {
            let mut el = self.rt.event_loop.lock().unwrap();
            let id = el.register_promise(
                Persistent::save(&ctx, resolve),
                Persistent::save(&ctx, reject)
            );
            (id, el.async_sender.clone())
        };

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

            let _ = sender.send(AsyncResult {
                id,
                result: final_result,
            });
        });

        Ok(())
    }
}

pub fn register(rt: &JsRuntime) -> rquickjs::Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            let global = ctx.globals();
            Class::<Response>::define(&global)?;
            
            global.set("__internal_fetch", rquickjs::Function::new(ctx.clone(), InternalFetch { rt: rt.clone() }))?;

            ctx.eval::<(), _>(r#"
                globalThis.fetch = function(url) {
                    return new Promise((resolve, reject) => {
                        __internal_fetch(url, resolve, reject);
                    });
                };
            "#)?;
            Ok(())
        })
    })
}
