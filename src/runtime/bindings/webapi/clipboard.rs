use rquickjs::{Class, Ctx, Result, Value, Object, Persistent, Function};
use crate::runtime::core::runtime::JsRuntime;
use std::sync::{Arc, Mutex};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Clipboard {
}

#[rquickjs::methods]
impl Clipboard {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    #[qjs(rename = "readText")]
    pub fn read_text<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let (promise, resolve, reject) = rquickjs::Promise::new(&ctx)?;
        
        let rt = ctx.userdata::<JsRuntime>().expect("JsRuntime").clone();
        
        tokio::task::spawn_blocking(move || {
            let mut clipboard = match arboard::Clipboard::new() {
                Ok(c) => c,
                Err(e) => {
                    let mut el = rt.event_loop.lock().unwrap();
                    el.queue_macro_task(move || {
                        rt.with_context(|ctx| {
                            ctx.with(|ctx| {
                                let _ = reject.call::<(String,), ()>((format!("Clipboard Error: {}", e),));
                            });
                        });
                    });
                    return;
                }
            };

            match clipboard.get_text() {
                Ok(text) => {
                    let mut el = rt.event_loop.lock().unwrap();
                    el.queue_macro_task(move || {
                        rt.with_context(|ctx| {
                            ctx.with(|ctx| {
                                let _ = resolve.call::<(String,), ()>((text,));
                            });
                        });
                    });
                }
                Err(e) => {
                    let mut el = rt.event_loop.lock().unwrap();
                    el.queue_macro_task(move || {
                        rt.with_context(|ctx| {
                            ctx.with(|ctx| {
                                let _ = reject.call::<(String,), ()>((format!("Clipboard Error: {}", e),));
                            });
                        });
                    });
                }
            }
        });

        Ok(promise.into_value())
    }

    #[qjs(rename = "writeText")]
    pub fn write_text<'js>(&self, ctx: Ctx<'js>, text: String) -> Result<Value<'js>> {
        let (promise, resolve, reject) = rquickjs::Promise::new(&ctx)?;
        
        let rt = ctx.userdata::<JsRuntime>().expect("JsRuntime").clone();

        tokio::task::spawn_blocking(move || {
            let mut clipboard = match arboard::Clipboard::new() {
                Ok(c) => c,
                Err(e) => {
                    let mut el = rt.event_loop.lock().unwrap();
                    el.queue_macro_task(move || {
                        rt.with_context(|ctx| {
                            ctx.with(|ctx| {
                                let _ = reject.call::<(String,), ()>((format!("Clipboard Error: {}", e),));
                            });
                        });
                    });
                    return;
                }
            };

            match clipboard.set_text(text) {
                Ok(_) => {
                    let mut el = rt.event_loop.lock().unwrap();
                    el.queue_macro_task(move || {
                        rt.with_context(|ctx| {
                            ctx.with(|ctx| {
                                let _ = resolve.call::<(), ()>(());
                            });
                        });
                    });
                }
                Err(e) => {
                    let mut el = rt.event_loop.lock().unwrap();
                    el.queue_macro_task(move || {
                        rt.with_context(|ctx| {
                            ctx.with(|ctx| {
                                let _ = reject.call::<(String,), ()>((format!("Clipboard Error: {}", e),));
                            });
                        });
                    });
                }
            }
        });

        Ok(promise.into_value())
    }
}
