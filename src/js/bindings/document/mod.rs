use rquickjs::{Ctx, Value, Class, Result};
use crate::engine::dom::{AceDOM, AceNodeType};
use crate::js::JsRuntime;
use crate::js::bindings::element::Element;
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
    pub dom: Arc<Mutex<AceDOM>>,
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

    #[qjs(get, rename = "title")]
    pub fn title(&self) -> String {
        let dom = self.dom.lock().unwrap();
        // Look for <title> text
        if let Some(head_idx) = dom.head {
            if let Some(head_node) = dom.get_node(head_idx) {
                for &child_idx in &head_node.children {
                    if let Some(child) = dom.get_node(child_idx) {
                        if let AceNodeType::Element(el) = &child.node_type {
                            if el.tag == "title" {
                                return dom.serialize_subtree(child_idx)
                                    .replace("<title>", "")
                                    .replace("</title>", "");
                            }
                        }
                    }
                }
            }
        }
        "".to_string()
    }

    #[qjs(set, rename = "title")]
    pub fn set_title(&self, title: String) {
        println!("Document title set to: {}", title);
        // For now, we don't mutate the DOM for title, but we could
    }

    #[qjs(rename = "createElement")]
    pub fn create_element<'js>(&self, ctx: Ctx<'js>, tag_name: String) -> Result<Value<'js>> {
        // TODO: Implement node creation in AceDOM
        // For now, return primitive dummy or fail?
        // AceDOM structure is mainly for parsing. 
        // We'll create a disconnected element.
        
        let mut dom = self.dom.lock().unwrap();
        // Create a basic Element node
        let node_type = AceNodeType::Element(crate::engine::dom::AceElement {
            tag: tag_name,
            attributes: std::collections::HashMap::new(),
        });
        
        let node_idx = dom.nodes.len();
        dom.nodes.push(crate::engine::dom::AceNode {
            node_type,
            parent: None,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
        });

        let element = Element { 
            dom: self.dom.clone(),
            index: node_idx,
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
        let dom = self.dom.lock().unwrap();
        if let Some(body_idx) = dom.body {
             let idx = body_idx;
             drop(dom);
             
             let element = Element { 
                dom: self.dom.clone(),
                index: idx,
                mutations: self.mutations.clone(),
                stylesheet_dirty: self.stylesheet_dirty.clone(),
            };
            let instance = Class::instance(ctx, element)?;
            return Ok(instance.into_value());
        }
        Ok(Value::new_null(ctx))
    }

    #[qjs(get, rename = "head")]
    pub fn head<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let dom = self.dom.lock().unwrap();
        if let Some(head_idx) = dom.head {
             let idx = head_idx;
             drop(dom);
             
             let element = Element { 
                dom: self.dom.clone(),
                index: idx,
                mutations: self.mutations.clone(),
                stylesheet_dirty: self.stylesheet_dirty.clone(),
            };
            let instance = Class::instance(ctx, element)?;
            return Ok(instance.into_value());
        }
        Ok(Value::new_null(ctx))
    }

    #[qjs(get, rename = "documentElement")]
    pub fn document_element<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let dom = self.dom.lock().unwrap();
        let root_idx = dom.root;
        drop(dom);
        
        let element = Element { 
            dom: self.dom.clone(),
            index: root_idx,
            mutations: self.mutations.clone(),
            stylesheet_dirty: self.stylesheet_dirty.clone(),
        };
        let instance = Class::instance(ctx, element)?;
        Ok(instance.into_value())
    }
}

fn get_computed_style_js<'js>(ctx: Ctx<'js>, val: Value<'js>) -> Result<Class<'js, crate::js::bindings::computed_style::ComputedCSSStyleDeclaration>> {
    let document: Class<Document> = ctx.globals().get("document")?;
    let styles = document.borrow().stylesheet.clone();
    
    let el = Class::<Element>::from_value(&val).map_err(|_| rquickjs::Error::new_from_js("Argument must be an Element", "TypeError"))?;
    
    // We need indices now, ComputedCSSStyleDeclaration needs update too maybe?
    // Let's assume ComputedCSSStyleDeclaration is updated or we pass needed info.
    // For now, pass indices. ComputedCSSStyleDeclaration likely needs Dom access.
    
    // This part involves computed_style.rs. We might break it here.
    // Let's comment out the implementation details for now or stub.
    
    let computed = crate::js::bindings::computed_style::ComputedCSSStyleDeclaration { 
        dom: el.borrow().dom.clone(),
        node_idx: el.borrow().index,
        stylesheet: styles
    };
    Class::instance(ctx, computed)
}

// Register document API in the runtime
pub fn register(rt: &JsRuntime, dom: Arc<Mutex<AceDOM>>, stylesheet: std::sync::Arc<std::sync::Mutex<crate::engine::style::Stylesheet>>) -> Result<()> {
    rt.with_context(|context| {
        context.with(|ctx| {
            // Register classes
            Class::<Element>::define(&ctx.globals())?;
            Class::<crate::js::bindings::token_list::DomTokenList>::define(&ctx.globals())?;
            Class::<crate::js::bindings::style_declaration::CssStyleDeclaration>::define(&ctx.globals())?;
            Class::<crate::js::bindings::computed_style::ComputedCSSStyleDeclaration>::define(&ctx.globals())?;
            Class::<crate::js::bindings::event::Event>::define(&ctx.globals())?;
            Class::<crate::js::bindings::mutation_observer::MutationObserver>::define(&ctx.globals())?;
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
