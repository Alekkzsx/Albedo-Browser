use rquickjs::{Ctx, Function, Persistent, prelude::*};
use crate::runtime::core::runtime::JsRuntime;

pub fn register(rt: &JsRuntime) -> rquickjs::Result<()> {
    rt.with_context(|context| {
        context.with(|ctx| {
            let global = ctx.globals();

            // setTimeout
            let rt_clone = rt.clone();
            let set_timeout = Function::new(ctx.clone(), move |callback: Function<'_>, delay: Option<f64>| -> rquickjs::Result<u32> {
                let delay = delay.unwrap_or(0.0) as u64;
                // Use the context associated with the callback to ensure identical lifetimes
                let cb_ctx = callback.ctx().clone();
                let callback_persistent = Persistent::save(&cb_ctx, callback);
                let mut event_loop = rt_clone.event_loop.lock().unwrap();
                Ok(event_loop.set_timer(callback_persistent, delay, false))
            })?;
            global.set("setTimeout", set_timeout)?;

            // requestAnimationFrame
            let rt_clone = rt.clone();
            let raf = Function::new(ctx.clone(), move |callback: Function<'_>| -> rquickjs::Result<u32> {
                let cb_ctx = callback.ctx().clone();
                let callback_persistent = Persistent::save(&cb_ctx, callback);
                let mut event_loop = rt_clone.event_loop.lock().unwrap();
                Ok(event_loop.set_timer(callback_persistent, 16, false))
            })?;
            global.set("requestAnimationFrame", raf)?;

            // clearTimeout
            let rt_clone = rt.clone();
            let clear_timeout = Function::new(ctx.clone(), move |id: u32| {
                let mut event_loop = rt_clone.event_loop.lock().unwrap();
                event_loop.clear_timer(id);
            })?;
            global.set("clearTimeout", clear_timeout)?;

            // setInterval
            let rt_clone = rt.clone();
            let set_interval = Function::new(ctx.clone(), move |callback: Function<'_>, delay: Option<f64>| -> rquickjs::Result<u32> {
                let delay = delay.unwrap_or(0.0) as u64;
                let cb_ctx = callback.ctx().clone();
                let callback_persistent = Persistent::save(&cb_ctx, callback);
                let mut event_loop = rt_clone.event_loop.lock().unwrap();
                Ok(event_loop.set_timer(callback_persistent, delay, true))
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
