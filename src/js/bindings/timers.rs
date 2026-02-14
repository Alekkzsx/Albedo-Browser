use rquickjs::{Ctx, Function, Persistent};
use crate::js::JsRuntime;

pub fn register(rt: &JsRuntime) -> rquickjs::Result<()> {
    rt.with_context(|context| {
        context.with(|ctx| {
            let global = ctx.globals();
            
            // setTimeout
            let rt_clone = rt.clone();
            let set_timeout = Function::new(ctx.clone(), move |callback: Function, delay: Option<u64>| {
                let delay = delay.unwrap_or(0);
                let ctx = callback.ctx().clone();
                let persistent = Persistent::save(&ctx, callback);
                let mut event_loop = rt_clone.event_loop.lock().unwrap();
                event_loop.set_timer(persistent, delay, false)
            })?;
            global.set("setTimeout", set_timeout)?;

            // clearTimeout
            let rt_clone = rt.clone();
            let clear_timeout = Function::new(ctx.clone(), move |id: u32| {
                let mut event_loop = rt_clone.event_loop.lock().unwrap();
                event_loop.clear_timer(id);
            })?;
            global.set("clearTimeout", clear_timeout)?;

            // setInterval
            let rt_clone = rt.clone();
            let set_interval = Function::new(ctx.clone(), move |callback: Function, delay: Option<u64>| {
                let delay = delay.unwrap_or(0);
                let ctx = callback.ctx().clone();
                let persistent = Persistent::save(&ctx, callback);
                let mut event_loop = rt_clone.event_loop.lock().unwrap();
                event_loop.set_timer(persistent, delay, true)
            })?;
            global.set("setInterval", set_interval)?;

            // clearInterval
            let rt_clone = rt.clone();
            let clear_interval = Function::new(ctx.clone(), move |id: u32| {
                let mut event_loop = rt_clone.event_loop.lock().unwrap();
                event_loop.clear_timer(id);
            })?;
            global.set("clearInterval", clear_interval)?;
            
            Ok(())
        })
    })
}
