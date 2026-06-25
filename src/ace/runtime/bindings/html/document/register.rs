use super::*;
use super::element::Element;
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use crate::ace::engine::style::Stylesheet;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};


/// TODO: add docs
pub fn register(
    rt: &JsRuntime,
    dom: Arc<Mutex<AceDOM>>,
    stylesheet: std::sync::Arc<std::sync::Mutex<crate::ace::engine::style::Stylesheet>>,
    primitives: Arc<Mutex<Vec<crate::ace::engine::ACEPrimitive>>>,
    canvas_contexts: Arc<
        Mutex<std::collections::HashMap<usize, crate::ace::engine::graphics::canvas2d::Canvas2D>>,
    >,
    url: String,
    referrer: String,
    resource_manager: Option<crate::network::resources::ResourceManager>,
) -> Result<()> {
    rt.with_context(|context| {
        context.with(|ctx| {
            let global = ctx.globals();
            // Register classes
            Class::<Element>::define(&global)?;
            Class::<Document>::define(&global)?;
            Class::<crate::ace::runtime::bindings::html::computed_style::ComputedCSSStyleDeclaration>::define(&global)?;

            let cookies = if let Some(rm) = resource_manager.as_ref() {
                rm.cookie_jar.lock().unwrap_or_else(|e| e.into_inner()).get_cookies_for_url(&url)
            } else {
                String::new()
            };

            let doc_instance = Class::instance(
                ctx.clone(),
                Document {
                    dom,
                    stylesheet: stylesheet.clone(),
                    mutations: rt.mutations.clone(),
                    stylesheet_dirty: rt.stylesheet_dirty.clone(),
                    cookie_storage: Arc::new(Mutex::new(cookies)),
                    resource_manager,
                    primitives,
                    canvas_contexts,
                    pending_scroll: rt.pending_scroll.clone(),
                    element_geometry: rt.element_geometry.clone(), // Add
                    element_scroll: rt.element_scroll.clone(),     // Add
                    ready_state_ptr: rt.ready_state.clone(),
                    url,
                    referrer,
                },
            )?;
            global.set("document", doc_instance)?;

            // Register global getComputedStyle function
            let get_computed_style = Function::new(ctx.clone(), |ctx: Ctx<'_>, val: Value<'_>| -> Result<crate::ace::runtime::bindings::html::computed_style::ComputedCSSStyleDeclaration> {
                let document: Class<Document> = ctx.globals().get("document")?;
                let doc = document.borrow();
                let dom = doc.dom.clone();
                let stylesheet = doc.stylesheet.clone();
                
                let el = Class::<Element>::from_value(&val).map_err(|_| rquickjs::Error::new_from_js("Argument must be an Element", "TypeError"))?;
                let node_idx = el.borrow().index;
                
                Ok(crate::ace::runtime::bindings::html::computed_style::ComputedCSSStyleDeclaration { 
                    dom, 
                    node_idx, 
                    stylesheet
                })
            })?;
            global.set("getComputedStyle", get_computed_style)?;

            Ok(())
        })
    })
}
