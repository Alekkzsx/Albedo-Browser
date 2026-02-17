use rquickjs::{Class, Ctx, Function, Result, Value, Object};
use crate::runtime::core::runtime::JsRuntime;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Clipboard {}

#[rquickjs::methods]
impl Clipboard {
    #[qjs(constructor)]
    pub fn new() -> Self { Self {} }

    #[qjs(rename = "writeText")]
    pub fn write_text<'js>(&self, ctx: Ctx<'js>, text: String) -> Result<Value<'js>> {
        let (promise, resolve, _reject) = rquickjs::Promise::new(&ctx)?;
        // Operation is synchronous here (stub); resolve immediately
        let _ = resolve.call::<(String,), ()>((text,));
        Ok(promise.into_value())
    }

    #[qjs(rename = "readText")]
    pub fn read_text<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let (promise, resolve, _reject) = rquickjs::Promise::new(&ctx)?;
        // Return stubbed clipboard content synchronously
        let _ = resolve.call::<(String,), ()>(("Clipboard content stub".to_string(),));
        Ok(promise.into_value())
    }
}

pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|ctx: &rquickjs::Context| {
        ctx.with(|ctx: Ctx<'_>| {
            let instance = Class::instance(ctx.clone(), Clipboard {})?;
            ctx.globals().get::<_, Object>("navigator")?.set("clipboard", instance)?;
            Ok(())
        })
    })
}
