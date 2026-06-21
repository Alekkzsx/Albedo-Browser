use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Result};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Performance {
    #[qjs(skip_trace)]
    page_start_time: std::time::Instant,
}

#[rquickjs::methods]
impl Performance {
    #[qjs(rename = "now")]
    pub fn now(&self) -> f64 {
        self.page_start_time.elapsed().as_secs_f64() * 1000.0
    }

    #[qjs(rename = "mark")]
    pub fn mark(&self, _name: String) {
        // Stub for performance.mark
    }

    #[qjs(rename = "measure")]
    pub fn measure(&self, _name: String, _start_mark: Option<String>, _end_mark: Option<String>) {
        // Stub for performance.measure
    }
}

pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            let global = ctx.globals();
            Class::<Performance>::define(&global)?;

            let perf = Performance {
                page_start_time: rt.page_start_time,
            };
            let instance = Class::instance(ctx.clone(), perf)?;
            global.set("performance", instance)?;
            Ok(())
        })
    })
}
