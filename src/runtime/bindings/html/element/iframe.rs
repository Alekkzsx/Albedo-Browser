/// HTML iframe element specific bindings (contentWindow, contentDocument, etc.)
use rquickjs::{Ctx, Value, Result};
use super::Element;

/// Get the contentWindow property of an iframe element
pub fn get_content_window<'js>(ctx: &Ctx<'js>, el: &Element) -> Result<Value<'js>> {
    let engines = el.subframe_engines.lock().unwrap();
    if let Some(engine_arc) = engines.get(&el.index) {
        let engine = engine_arc.lock().unwrap();
        if let Some(ref sub_rt) = engine.js_runtime {
            // SOP Check: Access to window is generally allowed but restricted.
            // However, typical Web APIs return the WindowProxy.
            let sub_ctx = sub_rt.context.lock().unwrap();
            // Returning cross-context values is not supported directly.
            // For now, return null. A Proxy should be implemented here in the future.
            return Ok(Value::new_null(ctx.clone()));
        }
    }
    Ok(Value::new_null(ctx.clone()))
}

/// Get the contentDocument property of an iframe element
pub fn get_content_document<'js>(ctx: &Ctx<'js>, el: &Element) -> Result<Value<'js>> {
    let caller_rt: crate::runtime::core::runtime::JsRuntime = ctx.globals().get("__albedo_rt__")?;
    
    let engines = el.subframe_engines.lock().unwrap();
    if let Some(engine_arc) = engines.get(&el.index) {
        let engine = engine_arc.lock().unwrap();
        if let Some(ref sub_rt) = engine.js_runtime {
            // SOP Check: contentDocument returns null if cross-origin
            if !caller_rt.check_same_origin(sub_rt) {
                println!("[SOP] Blocked cross-origin access to contentDocument");
                return Ok(Value::new_null(ctx.clone()));
            }

            // Document is already registered in the subframe's globals
            let sub_ctx = sub_rt.context.lock().unwrap();
            // Returning cross-context values is not supported directly.
            return Ok(Value::new_null(ctx.clone()));
        }
    }
    Ok(Value::new_null(ctx.clone()))
}

/// Get the src attribute of an iframe
pub fn get_iframe_src(el: &Element) -> Result<String> {
    let dom = el.dom.lock().expect("DOM lock failed");
    
    if let Some(node) = dom.get_node(el.index) {
        if let crate::engine::dom::AceNodeType::Element(ace_el) = &node.node_type {
            if ace_el.is_iframe() {
                return Ok(ace_el.attributes.get("src").cloned().unwrap_or_default());
            }
        }
    }
    
    Ok(String::new())
}
