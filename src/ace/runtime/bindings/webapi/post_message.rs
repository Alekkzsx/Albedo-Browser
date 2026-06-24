use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{prelude::This, Ctx, Function, Result, Value};

/// postMessage(message, targetOrigin, [transfer])
pub fn post_message<'js>(
    ctx: Ctx<'js>,
    This(this): This<Value<'js>>,
    message: Value<'js>,
    target_origin: String,
    _transfer: Option<Value<'js>>,
) -> Result<()> {
    let caller_rt: JsRuntime = ctx.globals().get("__albedo_rt__")?;

    // Determine the target runtime.
    let mut target_rt_id = caller_rt.id;

    if let Some(obj) = this.as_object() {
        if let Ok(rt_instance) = obj.get::<_, JsRuntime>("__albedo_rt__") {
            target_rt_id = rt_instance.id;
        }
    }

    let message_json = ctx
        .json_stringify(message)?
        .ok_or_else(|| rquickjs::Error::Exception)?
        .as_string()
        .expect("Albedo Engine: internal invariant violated")
        .to_string()?;

    let caller_origin = caller_rt
        .origin
        .lock().unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map(|o: &crate::network::security::Origin| o.to_string())
        .unwrap_or_else(|| "null".to_string());

    if let Some(target_rt_arc) = crate::ace::runtime::core::registry::get_runtime(target_rt_id) {
        let target_rt = target_rt_arc.lock().unwrap_or_else(|e| e.into_inner());

        // Origin Check
        let mut allowed = true;
        if target_origin == "/" {
            // Same-origin only
            allowed = caller_rt.check_same_origin(&target_rt);
        } else if target_origin != "*" {
            let target_rt_origin_lock = target_rt.origin.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref target_rt_origin) = *target_rt_origin_lock {
                if target_rt_origin.to_string() != target_origin {
                    allowed = false;
                }
            } else if target_origin != "null" {
                allowed = false;
            }
        }

        if !allowed {
            tracing::warn!("postMessage blocked: origin mismatch");
            return Ok(());
        }

        // Dispatch in target runtime
        target_rt.dispatch_message_event(message_json, caller_origin, Some(caller_rt.id));
    }

    Ok(())
}

/// TODO: add docs
pub fn register(ctx: &Ctx<'_>) -> Result<()> {
    let global = ctx.globals();
    global.set("postMessage", Function::new(ctx.clone(), post_message)?)?;
    Ok(())
}
