use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Result, Value};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct AbortSignal {
    #[qjs(skip_trace)]
    pub aborted_internal: std::sync::Arc<std::sync::Mutex<bool>>,
}

#[rquickjs::methods]
impl AbortSignal {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            aborted_internal: std::sync::Arc::new(std::sync::Mutex::new(false)),
        }
    }

    #[qjs(get, rename = "aborted")]
    pub fn aborted(&self) -> bool {
        *self.aborted_internal.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[qjs(rename = "_abort")]
    pub fn _abort(&self) {
        let mut aborted = self.aborted_internal.lock().unwrap_or_else(|e| e.into_inner());
        *aborted = true;
    }

    #[qjs(rename = "addEventListener")]
    pub fn add_event_listener<'js>(&self, _type_: String, _listener: Function<'js>) {}

    #[qjs(rename = "removeEventListener")]
    pub fn remove_event_listener<'js>(&self, _type_: String, _listener: Function<'js>) {}
}

/// TODO: add docs
pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|context| {
        context.with(|ctx| {
            let global = ctx.globals();

            // Expose the AbortSignal class but rename it to hide the constructor,
            // actually we can just expose AbortSignal
            Class::<AbortSignal>::define(&global)?;

            // AbortController
            ctx.eval::<(), _>(
                r#"
                globalThis.AbortController = class AbortController {
                    constructor() {
                        this.signal = new AbortSignal();
                    }
                    abort() {
                        this.signal._abort();
                        // Dispatch abort event
                        if (typeof this.signal.onabort === 'function') {
                            this.signal.onabort();
                        }
                    }
                };
            "#,
            )?;

            Ok(())
        })
    })
}
