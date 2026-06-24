use rquickjs::{Class, Ctx, Object, Result, Value};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Notification {
    pub title: String,
    pub body: String,
}

#[rquickjs::methods]
impl Notification {
    #[qjs(constructor)]
    pub fn new(_ctx: Ctx<'_>, title: String, options: Option<Object<'_>>) -> Self {
        let mut body = String::new();
        if let Some(opts) = options {
            body = opts.get("body").unwrap_or_default();
        }

        let _ = notify_rust::Notification::new()
            .summary(&title)
            .body(&body)
            .show();

        Self { title, body }
    }

    #[qjs(static)]
    #[qjs(static, rename = "requestPermission")]
    pub fn request_permission<'js>(ctx: Ctx<'js>) -> Result<Value<'js>> {
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ = resolve.call::<(&str,), ()>(("granted",));
        Ok(promise.into_value())
    }

    #[qjs(static, get)]
    pub fn permission() -> String {
        "granted".to_string()
    }

    #[qjs(get)]
    pub fn title(&self) -> String {
        self.title.clone()
    }

    #[qjs(get)]
    pub fn body(&self) -> String {
        self.body.clone()
    }
}

/// TODO: add docs
pub fn register(ctx: &rquickjs::Context) -> Result<()> {
    ctx.with(|ctx| {
        let globals = ctx.globals();
        globals.set("Notification", Class::<Notification>::register(&ctx)?)?;
        Ok(())
    })
}
