use super::*;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{prelude::*, ArrayBuffer, Class, Ctx, Function, Object, Persistent, Result, Value};
// No UnsafeSendVal needed here after synchronous refactor
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn register(rt: &JsRuntime) -> rquickjs::Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            let globals = ctx.globals();
            globals.set("Blob", Class::<Blob>::register(&ctx)?)?;
            globals.set("File", Class::<File>::register(&ctx)?)?;
            globals.set("FileReader", Class::<FileReader>::register(&ctx)?)?;
            Ok(())
        })
    })
}
