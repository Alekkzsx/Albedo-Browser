use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Result};
use std::sync::{Arc, Mutex};



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
    #[qjs(get, rename = "availWidth")]
    pub fn avail_width(&self) -> i32 {
        self.size.lock().unwrap().0
    }
    #[qjs(get, rename = "availHeight")]
    pub fn avail_height(&self) -> i32 {
        self.size.lock().unwrap().1
    }
    #[qjs(get, rename = "colorDepth")]
    pub fn color_depth(&self) -> i32 {
        24
    }
    #[qjs(get, rename = "pixelDepth")]
    pub fn pixel_depth(&self) -> i32 {
        24
    }
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Performance {}

#[rquickjs::methods]
impl Performance {
    pub fn now(&self) -> f64 {
        crate::utils::time::monotonic_now()
    }
}

pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|context| {
        context.with(|ctx| {
            let global = ctx.globals();

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
