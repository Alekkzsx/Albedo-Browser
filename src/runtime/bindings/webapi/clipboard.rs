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
        let (promise, resolve, reject) = rquickjs::Promise::new(&ctx)?;
        
        match arboard::Clipboard::new() {
            Ok(mut cb) => {
                if cb.set_text(text.clone()).is_ok() {
                    let _ = resolve.call::<(String,), ()>((text,));
                } else {
                    let _ = reject.call::<(String,), ()>(("Falha ao escrever no clipboard".to_string(),));
                }
            },
            Err(_) => {
                let _ = reject.call::<(String,), ()>(("Falha ao abrir clipboard".to_string(),));
            }
        }
        
        Ok(promise.into_value())
    }

    #[qjs(rename = "readText")]
    pub fn read_text<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let (promise, resolve, reject) = rquickjs::Promise::new(&ctx)?;
        
        match arboard::Clipboard::new() {
            Ok(mut cb) => {
                if let Ok(texto) = cb.get_text() {
                    let _ = resolve.call::<(String,), ()>((texto,));
                } else {
                    let _ = resolve.call::<(String,), ()>(("".to_string(),));
                }
            },
            Err(_) => {
                let _ = reject.call::<(String,), ()>(("Falha ao ler clipboard".to_string(),));
            }
        }
        
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
