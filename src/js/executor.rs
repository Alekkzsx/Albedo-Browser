use crate::js::JsRuntime;
use rquickjs::{Value, Ctx};

pub fn run_pending(rt: &JsRuntime) -> (bool, bool) {
    let mut executed = false;
    
    // 0. Check for DOM mutations that happened since last pulse
    {
        let mut muts = rt.mutations.lock().unwrap();
        if *muts {
            executed = true;
            *muts = false;
        }
    }

    // 1. Run QuickJS pending jobs (Promises/microtasks)
    {
        let ctx = rt.context.lock().unwrap();
        ctx.with(|ctx| {
            if ctx.execute_pending_job() {
                println!("[JS] Microtask/Promise executed.");
                executed = true;
            }
        });
    }

    // 2. Handle Async Bridge (Fetch, etc)
    let async_results = {
        let mut el = rt.event_loop.lock().unwrap();
        el.receive_async_results()
    };

    if !async_results.is_empty() {
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                let mut el = rt.event_loop.lock().unwrap();
                for res in async_results {
                    if let Some(resolution) = el.take_resolution(res.id) {
                        match res.result {
                            Ok((status, body)) => {
                                if let Ok(resolve) = resolution.resolve.restore(&ctx) {
                                    use crate::js::bindings::fetch::Response;
                                    let response = Response { status, body };
                                    if let Ok(instance) = rquickjs::Class::instance(ctx.clone(), response) {
                                        let _: rquickjs::Result<()> = resolve.call((instance,));
                                        executed = true;
                                    }
                                }
                            }
                            Err(err) => {
                                if let Ok(reject) = resolution.reject.restore(&ctx) {
                                    let _: rquickjs::Result<()> = reject.call((err,));
                                    executed = true;
                                }
                            }
                        }
                    }
                }
                // Run jobs again as resolutions might trigger then() callbacks
                if ctx.execute_pending_job() {
                    executed = true;
                }
            })
        });
    }

    // 3. Run EventLoop tasks (timers, etc)
    let (timers, macros) = {
        let mut el = rt.event_loop.lock().unwrap();
        el.take_pending_tasks()
    };
    
    if !timers.is_empty() {
            rt.with_context(|ctx| {
            ctx.with(|ctx| {
                for timer in timers {
                        if let Ok(func) = timer.callback.restore(&ctx) {
                        let _: rquickjs::Result<Value> = func.call(());
                        executed = true;
                    }
                }
                
                // Run pending jobs AGAIN after timers might have resolved promises
                if ctx.execute_pending_job() {
                    executed = true;
                }
            })
            });
    }
            
    for task in macros {
        task();
        executed = true;
    }

    // 4. Stylesheet dirty check
    let mut stylesheet_dirty = false;
    if let Ok(mut sd) = rt.stylesheet_dirty.lock() {
        if *sd {
            stylesheet_dirty = true;
            *sd = false;
        }
    }
    
    (executed, stylesheet_dirty)
}
