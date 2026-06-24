/// HTML iframe element specific bindings (contentWindow, contentDocument, etc.)
use rquickjs::{Ctx, Value, Result};
use super::Element;

/// Get the contentWindow property of an iframe element
pub fn get_content_window<'js>(ctx: &Ctx<'js>, el: &Element) -> Result<Value<'js>> {
    let caller_rt: crate::ace::runtime::core::runtime::JsRuntime = ctx.globals().get("__albedo_rt__")?;
    
    // In Albedo, subframe engine creation happens in the Engine. We need to get the engine references.
    // For now, if the Element struct was supposed to hold it but doesn't, we need to locate where `subframe_engines` was defined.
    // Wait, let's look at `AceDOM`.
    let dom = el.dom.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(subframes) = &dom.subframes {
         if let Some(engine_arc) = subframes.get(&el.index) {
             let engine = engine_arc.lock().unwrap_or_else(|e| e.into_inner());
             if let Some(ref sub_rt) = engine.js_runtime {
                 use crate::ace::runtime::bindings::webapi::window_proxy::WindowProxy;
                 if let Ok(proxy_val) = rquickjs::Class::instance(ctx.clone(), WindowProxy::new(sub_rt.id)) {
                     return Ok(proxy_val.into_value());
                 }
             }
         }
    }
    
    Ok(Value::new_null(ctx.clone()))
}

/// Get the contentDocument property of an iframe element
pub fn get_content_document<'js>(ctx: &Ctx<'js>, el: &Element) -> Result<Value<'js>> {
    let caller_rt: crate::ace::runtime::core::runtime::JsRuntime = ctx.globals().get("__albedo_rt__")?;
    
    let dom = el.dom.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(subframes) = &dom.subframes {
        if let Some(engine_arc) = subframes.get(&el.index) {
            let engine = engine_arc.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref sub_rt) = engine.js_runtime {
                // SOP Check: contentDocument returns null if cross-origin
                if !caller_rt.check_same_origin(sub_rt) {
                    tracing::warn!("SOP: Blocked cross-origin access to contentDocument");
                    return Ok(Value::new_null(ctx.clone()));
                }

                // WindowProxy is returned here actually as per some specs, or the true Document proxy
                return Ok(Value::new_null(ctx.clone()));
            }
        }
    }
    Ok(Value::new_null(ctx.clone()))
}

/// Get the src attribute of an iframe
pub fn get_iframe_src(el: &Element) -> Result<String> {
    let dom = el.dom.lock().expect("DOM lock failed");
    
    if let Some(node) = dom.get_node(el.index) {
        if let crate::ace::engine::dom::AceNodeType::Element(ace_el) = &node.node_type {
            if ace_el.is_iframe() {
                return Ok(ace_el.attributes.get("src").cloned().unwrap_or_default());
            }
        }
    }
    
    Ok(String::new())
}
