use rquickjs::{Class, Ctx, Function, Result, Value, prelude::*};
use crate::runtime::core::runtime::JsRuntime;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Worker {
    #[qjs(skip_trace)]
    pub(crate) rt: JsRuntime,
}

#[rquickjs::methods]
impl Worker {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>, url: String) -> Result<Self> {
        let rt_val = ctx.globals().get::<_, Value>("__albedo_rt__")?;
        let _parent_rt = Class::<JsRuntime>::from_object(rt_val.as_object().unwrap()).unwrap().borrow().clone();
        

        let rt = JsRuntime::new().map_err(|_| rquickjs::Error::new_from_js("Worker", "Failed to create worker runtime"))?;
        let worker_rt = rt.clone();
        
        rt.with_context(move |ctx: &rquickjs::Context| {
            ctx.with(|ctx: Ctx<'_>| {
                let _ = crate::runtime::bindings::utils::console::Console::register(&worker_rt);
                let _ = crate::runtime::bindings::webapi::timers::register(&worker_rt);
                
                let post_message = Function::new(ctx.clone(), move |_: Value<'_>| {
                    // Stub
                }).unwrap();
                ctx.globals().set("postMessage", post_message).unwrap();
            });
        });

        Ok(Self { rt })
    }

    pub fn post_message(&self, _msg: Value<'_>) {}
    pub fn terminate(&self) {}

    #[qjs(get, rename = "onmessage")]
    pub fn onmessage_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> { Ok(Value::new_null(ctx)) }
    #[qjs(set, rename = "onmessage")]
    pub fn onmessage_setter<'js>(&self, _f: Function<'js>) {}

    #[qjs(get, rename = "onerror")]
    pub fn onerror_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> { Ok(Value::new_null(ctx)) }
    #[qjs(set, rename = "onerror")]
    pub fn onerror_setter<'js>(&self, _f: Function<'js>) {}
}

pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            ctx.globals().set("Worker", Class::<Worker>::register(&ctx)?)?;
            Ok(())
        })
    })
}
