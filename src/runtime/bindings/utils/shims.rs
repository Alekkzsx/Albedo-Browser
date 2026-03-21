use crate::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Result};
use std::sync::{Arc, Mutex};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct History {}

#[rquickjs::methods]
impl History {
    #[qjs(get)]
    pub fn length(&self) -> i32 {
        1
    }

    pub fn pushState(&self) {
        // Stub
    }

    pub fn replaceState(&self) {
        // Stub
    }

    pub fn back(&self) {}
    pub fn forward(&self) {}
    pub fn go(&self) {}
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Screen {
    #[qjs(skip_trace)]
    pub size: Arc<Mutex<(i32, i32)>>,
}

#[rquickjs::methods]
impl Screen {
    #[qjs(get)]
    pub fn width(&self) -> i32 {
        self.size.lock().unwrap().0
    }
    #[qjs(get)]
    pub fn height(&self) -> i32 {
        self.size.lock().unwrap().1
    }
    #[qjs(get)]
    pub fn availWidth(&self) -> i32 {
        self.size.lock().unwrap().0
    }
    #[qjs(get)]
    pub fn availHeight(&self) -> i32 {
        self.size.lock().unwrap().1
    }
    #[qjs(get)]
    pub fn colorDepth(&self) -> i32 {
        24
    }
    #[qjs(get)]
    pub fn pixelDepth(&self) -> i32 {
        24
    }
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Performance {}

#[rquickjs::methods]
impl Performance {
    pub fn now(&self) -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
            * 1000.0
    }
}

pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|context| {
        context.with(|ctx| {
            let global = ctx.globals();

            let history = Class::instance(ctx.clone(), History {})?;
            global.set("history", history)?;

            let screen = Class::instance(
                ctx.clone(),
                Screen {
                    size: rt.screen_size.clone(),
                },
            )?;
            global.set("screen", screen)?;

            let performance = Class::instance(ctx.clone(), Performance {})?;
            global.set("performance", performance)?;

            Ok(())
        })
    })
}
