use rquickjs::{Context, Result, Class, Ctx};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Navigator {
}

#[rquickjs::methods]
impl Navigator {
    #[qjs(get, rename = "userAgent")]
    pub fn user_agent(&self) -> String {
        // Imitate Chrome to avoid being blocked or served legacy content
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string()
    }

    #[qjs(get)]
    pub fn platform(&self) -> String {
        "Linux x86_64".to_string()
    }

    #[qjs(get)]
    pub fn language(&self) -> String {
        "en-US".to_string()
    }
    
    #[qjs(get, rename = "cookieEnabled")]
    pub fn cookie_enabled(&self) -> bool {
        true
    }
    
    #[qjs(get)]
    pub fn onLine(&self) -> bool {
        true
    }
}

pub fn register(ctx: &Context) -> Result<()> {
    ctx.with(|ctx| {
        let global = ctx.globals();
        let navigator = Class::instance(ctx.clone(), Navigator {})?;
        global.set("navigator", navigator)?;
        Ok(())
    })
}
