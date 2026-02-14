use rquickjs::{Ctx, Value, Class, Result};
use crate::engine::dom::DomTree;
use crate::js::JsRuntime;
use crate::js::bindings::element::Element;
use kuchiki::NodeRef;
use html5ever::{QualName, LocalName, ns, namespace_url};
use std::sync::{Arc, Mutex};
use crate::engine::style::Stylesheet;
use crate::js::bindings::event::EventTargetImpl;
use rquickjs::Function;

pub mod query;
pub mod events;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Document {
    #[qjs(skip_trace)]
    pub dom: DomTree,
    #[qjs(skip_trace)]
    pub stylesheet: Arc<Mutex<Stylesheet>>,
    #[qjs(skip_trace)]
    pub mutations: Arc<Mutex<bool>>,
    #[qjs(skip_trace)]
    pub stylesheet_dirty: Arc<Mutex<bool>>,
    #[qjs(skip_trace)]
    pub cookie_storage: Arc<Mutex<String>>,
}

#[rquickjs::methods]
impl Document {
    #[qjs(rename = "getElementById")]
    pub fn get_element_by_id<'js>(&self, ctx: Ctx<'js>, id: String) -> Result<Value<'js>> {
        self::query::get_element_by_id(self, ctx, id)
    }

    #[qjs(rename = "addEventListener")]
    pub fn add_event_listener<'js>(&self, type_: String, listener: Function<'js>) {
        self::events::add_event_listener(self, type_, listener)
    }

    #[qjs(rename = "removeEventListener")]
    pub fn remove_event_listener<'js>(&self, type_: String, listener: Function<'js>) {
        self::events::remove_event_listener(self, type_, listener)
    }

    #[qjs(rename = "dispatchEvent")]
    pub fn dispatch_event<'js>(&self, event: Value<'js>) -> bool {
        self::events::dispatch_event(self, event)
    }

    #[qjs(get)]
    pub fn location<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        ctx.globals().get("location")
    }
    
    #[qjs(get, rename = "cookie")]
    pub fn cookie(&self) -> String {
        self.cookie_storage.lock().unwrap().clone()
    }

    #[qjs(set, rename = "cookie")]
    pub fn set_cookie(&self, val: String) {
        println!("Cookie set: {}", val);
        *self.cookie_storage.lock().unwrap() = val;
    }

    #[qjs(rename = "createElement")]
    pub fn create_element<'js>(&self, ctx: Ctx<'js>, tag_name: String) -> Result<Value<'js>> {
        let qual_name = QualName::new(None, ns!(html), LocalName::from(tag_name.as_str()));
        let node = NodeRef::new_element(qual_name, vec![]);
        
        let element = Element { 
            node,
            mutations: self.mutations.clone(),
            stylesheet_dirty: self.stylesheet_dirty.clone(),
        };
        let instance = Class::instance(ctx, element)?;
        Ok(instance.into_value())
    }

    #[qjs(rename = "querySelector")]
    pub fn query_selector<'js>(&self, ctx: Ctx<'js>, selector: String) -> Result<Value<'js>> {
        self::query::query_selector(self, ctx, selector)
    }

    #[qjs(rename = "querySelectorAll")]
    pub fn query_selector_all<'js>(&self, ctx: Ctx<'js>, selector: String) -> Result<Value<'js>> {
        self::query::query_selector_all(self, ctx, selector)
    }

    #[qjs(get, rename = "body")]
    pub fn body<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Ok(mut match_iter) = self.dom.root.select("body") {
            if let Some(node_match) = match_iter.next() {
                let element = Element { 
                    node: node_match.as_node().clone(),
                    mutations: self.mutations.clone(),
                    stylesheet_dirty: self.stylesheet_dirty.clone(),
                };
                let instance = Class::instance(ctx, element)?;
                return Ok(instance.into_value());
            }
        }
        Ok(Value::new_null(ctx))
    }
}

fn get_computed_style_js<'js>(ctx: Ctx<'js>, val: Value<'js>) -> Result<Class<'js, crate::js::bindings::computed_style::ComputedCSSStyleDeclaration>> {
    let document: Class<Document> = ctx.globals().get("document")?;
    let styles = document.borrow().stylesheet.clone();
    
    let el = Class::<Element>::from_value(&val).map_err(|_| rquickjs::Error::new_from_js("Argument must be an Element", "TypeError"))?;
    let node = el.borrow().node.clone();
    let computed = crate::js::bindings::computed_style::ComputedCSSStyleDeclaration { 
        node, 
        stylesheet: styles
    };
    Class::instance(ctx, computed)
}

// Register document API in the runtime
pub fn register(rt: &JsRuntime, dom: DomTree, stylesheet: std::sync::Arc<std::sync::Mutex<crate::engine::style::Stylesheet>>) -> Result<()> {
    rt.with_context(|context| {
        context.with(|ctx| {
            // Register classes
            Class::<Element>::define(&ctx.globals())?;
            Class::<crate::js::bindings::token_list::DomTokenList>::define(&ctx.globals())?;
            Class::<crate::js::bindings::style_declaration::CssStyleDeclaration>::define(&ctx.globals())?;
            Class::<crate::js::bindings::computed_style::ComputedCSSStyleDeclaration>::define(&ctx.globals())?;
            Class::<crate::js::bindings::event::Event>::define(&ctx.globals())?;
            Class::<Document>::define(&ctx.globals())?;
            
            // Create instance and set as global 'document'
            let doc_instance = Class::instance(ctx.clone(), Document { 
                dom, 
                stylesheet: stylesheet.clone(),
                mutations: rt.mutations.clone(),
                stylesheet_dirty: rt.stylesheet_dirty.clone(),
                cookie_storage: Arc::new(Mutex::new(String::new())),
            })?;
            ctx.globals().set("document", doc_instance)?;
            
            let get_computed_style = Function::new(ctx.clone(), get_computed_style_js)?;
            ctx.globals().set("getComputedStyle", get_computed_style)?;
            
            Ok(())
        })
    })
}


#[cfg(test)]
mod tests;
