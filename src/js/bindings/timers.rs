use rquickjs::{Ctx, Function, Persistent};
use crate::js::JsRuntime;

pub fn register(rt: &JsRuntime) -> rquickjs::Result<()> {
    rt.with_context(|context| {
        context.with(|ctx| {
            let global = ctx.globals();
            
            // setTimeout
            let rt_clone = rt.clone();
            let set_timeout = Function::new(ctx.clone(), move |ctx: Ctx, callback: Function, delay: Option<f64>, args: rquickjs::prelude::Rest<Value>| -> rquickjs::Result<u32> {
                let delay = delay.unwrap_or(0.0) as u64;
                
                // Wrap callback to include args
                let args_persistent: Vec<Persistent<Value>> = args.0.into_iter()
                    .map(|v| Persistent::save(&ctx, v))
                    .collect();
                
                let callback_persistent = Persistent::save(&ctx, callback);
                
                let wrapper = move |ctx: &Ctx| {
                    if let Some(cb) = callback_persistent.clone().restore(ctx) {
                        let actual_args: Vec<Value> = args_persistent.iter()
                            .filter_map(|p| p.clone().restore(ctx))
                            .collect();
                        let _: rquickjs::Value = cb.call(rquickjs::prelude::Rest(actual_args))?;
                    }
                    Ok::<(), rquickjs::Error>(())
                };

                let mut event_loop = rt_clone.event_loop.lock().unwrap();
                Ok(event_loop.set_timer_boxed(Box::new(wrapper), delay, false))
            })?;
            global.set("setTimeout", set_timeout)?;

            // requestAnimationFrame (alias to setTimeout(fn, 16) for now, or true v-sync if possible)
            let rt_clone = rt.clone();
            let raf = Function::new(ctx.clone(), move |ctx: Ctx, callback: Function| -> rquickjs::Result<u32> {
                let callback_persistent = Persistent::save(&ctx, callback);
                let wrapper = move |ctx: &Ctx| {
                    if let Some(cb) = callback_persistent.clone().restore(ctx) {
                        let _: rquickjs::Value = cb.call((ctx.globals().get::<_, f64>("performance").and_then(|p: rquickjs::Object| p.get("now")).unwrap_or(0.0),))?;
                    }
                    Ok::<(), rquickjs::Error>(())
                };
                let mut event_loop = rt_clone.event_loop.lock().unwrap();
                Ok(event_loop.set_timer_boxed(Box::new(wrapper), 16, false))
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
            let set_interval = Function::new(ctx.clone(), move |ctx: Ctx, callback: Function, delay: Option<f64>, args: rquickjs::prelude::Rest<Value>| -> rquickjs::Result<u32> {
                let delay = delay.unwrap_or(0.0) as u64;
                
                let args_persistent: Vec<Persistent<Value>> = args.0.into_iter()
                    .map(|v| Persistent::save(&ctx, v))
                    .collect();
                
                let callback_persistent = Persistent::save(&ctx, callback);
                
                let wrapper = move |ctx: &Ctx| {
                    if let Some(cb) = callback_persistent.clone().restore(ctx) {
                        let actual_args: Vec<Value> = args_persistent.iter()
                            .filter_map(|p| p.clone().restore(ctx))
                            .collect();
                        let _: rquickjs::Value = cb.call(rquickjs::prelude::Rest(actual_args))?;
                    }
                    Ok::<(), rquickjs::Error>(())
                };

                let mut event_loop = rt_clone.event_loop.lock().unwrap();
                Ok(event_loop.set_timer_boxed(Box::new(wrapper), delay, true))
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
