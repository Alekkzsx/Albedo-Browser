use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Result, Value};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct WindowProxy {
    #[qjs(skip_trace)]
    pub target_runtime_id: usize,
}

#[rquickjs::methods]
impl WindowProxy {
    #[qjs(constructor)]
    pub fn new(target_runtime_id: usize) -> Self {
        Self { target_runtime_id }
    }

    pub fn post_message<'js>(
        &self,
        ctx: Ctx<'js>,
        message: Value<'js>,
        target_origin: String,
        _transfer: Option<Value<'js>>,
    ) -> Result<()> {
        let caller_rt: JsRuntime = ctx.globals().get("__albedo_rt__")?;

        let message_json = match ctx.json_stringify(message)? {
            Some(v) => v.as_string().unwrap().to_string()?,
            None => "null".to_string(),
        };

        let caller_origin = caller_rt
            .origin
            .lock()
            .unwrap()
            .as_ref()
            .map(|o: &crate::network::security::Origin| o.to_string())
            .unwrap_or_else(|| "null".to_string());

        // Check origin and enqueue the message - all locks released before enqueue
        let (allowed, target_rt_arc) = {
            if let Some(arc) = crate::ace::runtime::core::registry::get_runtime(self.target_runtime_id) {
                let target_rt = arc.lock().unwrap();

                let mut allowed = true;
                if target_origin == "/" {
                    allowed = caller_rt.check_same_origin(&target_rt);
                } else if target_origin != "*" {
                    let target_rt_origin_lock = target_rt.origin.lock().unwrap();
                    if let Some(ref o) = *target_rt_origin_lock {
                        if o.to_string() != target_origin {
                            allowed = false;
                        }
                    } else if target_origin != "null" {
                        allowed = false;
                    }
                }
                (allowed, Some(arc.clone()))
            } else {
                (false, None)
            }
        }; // All locks on target_rt released here

        if !allowed {
            println!("[WindowProxy::postMessage] Blocked: origin mismatch.");
            return Ok(());
        }

        if let Some(arc) = target_rt_arc {
            // Enqueue the message in the TARGET runtime's event loop.
            // This only locks the event_loop mutex (not the JS context) - no deadlock.
            // The message is processed on the next run_pending() call in the target runtime.
            let target_rt = arc.lock().unwrap();
            target_rt.event_loop.lock().unwrap().enqueue_message(
                message_json,
                caller_origin,
                Some(caller_rt.id),
            );
        }

        Ok(())
    }
}

pub fn register(ctx: &Ctx<'_>) -> Result<()> {
    let global = ctx.globals();
    Class::<WindowProxy>::define(&global)?;
    Ok(())
}
