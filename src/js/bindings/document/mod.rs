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
pub mod collections;
pub mod fragment;

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
    #[qjs(skip_trace)]
    pub primitives: Arc<Mutex<Vec<crate::engine::ACEPrimitive>>>,
    pub url: String,
    pub referrer: String,
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

    #[qjs(get, rename = "defaultView")]
    pub fn default_view<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(ctx.globals().into_value())
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
    pub fn create_element<'js>(&self, ctx: Ctx<'js>, tag: String) -> Result<Value<'js>> {
        let idx = if let Ok(mut dom) = self.dom.lock() {
            let idx = dom.nodes.len();
            dom.nodes.push(AceNode {
                node_type: AceNodeType::Element(crate::engine::dom::AceElement { tag, attributes: std::collections::HashMap::new() }),
                parent: None,
                children: Vec::new(),
                prev_sibling: None,
                next_sibling: None,
                shadow_root: None,
            });
            idx
        } else {
            return Ok(Value::new_null(ctx));
        };

        let element = Element {
            dom: self.dom.clone(),
            index: idx,
            mutations: self.mutations.clone(),
            stylesheet_dirty: self.stylesheet_dirty.clone(),
            primitives: self.primitives.clone(),
        };

        let instance = Class::instance(ctx, element)?;
        Ok(instance.into_value())
    }

    #[qjs(rename = "createDocumentFragment")]
    pub fn create_document_fragment<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let idx = if let Ok(mut dom) = self.dom.lock() {
            let idx = dom.nodes.len();
            dom.nodes.push(AceNode {
                node_type: AceNodeType::DocumentFragment,
                parent: None,
                children: Vec::new(),
                prev_sibling: None,
                next_sibling: None,
                shadow_root: None,
            });
            idx
        } else {
            return Ok(Value::new_null(ctx));
        };

        let frag = crate::js::bindings::document::fragment::DocumentFragment {
            dom: self.dom.clone(),
            index: idx,
            mutations: self.mutations.clone(),
            stylesheet_dirty: self.stylesheet_dirty.clone(),
            primitives: self.primitives.clone(),
        };

        let instance = Class::instance(ctx, frag)?;
        Ok(instance.into_value())
    }

    #[qjs(rename = "createTextNode")]
    pub fn create_text_node<'js>(&self, ctx: Ctx<'js>, text: String) -> Result<Value<'js>> {
        let idx = if let Ok(mut dom) = self.dom.lock() {
            let idx = dom.nodes.len();
            dom.nodes.push(AceNode {
                node_type: AceNodeType::Text(text),
                parent: None,
                children: Vec::new(),
                prev_sibling: None,
                next_sibling: None,
                shadow_root: None,
            });
            idx
        } else {
            return Ok(Value::new_null(ctx));
        };

        let element = Element {
            dom: self.dom.clone(),
            index: idx,
            mutations: self.mutations.clone(),
            stylesheet_dirty: self.stylesheet_dirty.clone(),
            primitives: self.primitives.clone(),
        };

        let instance = Class::instance(ctx, element)?;
        Ok(instance.into_value())
    }

    #[qjs(rename = "createComment")]
    pub fn create_comment<'js>(&self, ctx: Ctx<'js>, data: String) -> Result<Value<'js>> {
        let idx = if let Ok(mut dom) = self.dom.lock() {
            let idx = dom.nodes.len();
            dom.nodes.push(AceNode {
                node_type: AceNodeType::Comment(data),
                parent: None,
                children: Vec::new(),
                prev_sibling: None,
                next_sibling: None,
                shadow_root: None,
            });
            idx
        } else {
            return Ok(Value::new_null(ctx));
        };

        let element = Element {
            dom: self.dom.clone(),
            index: idx,
            mutations: self.mutations.clone(),
            stylesheet_dirty: self.stylesheet_dirty.clone(),
            primitives: self.primitives.clone(),
        };

        let instance = Class::instance(ctx, element)?;
        Ok(instance.into_value())
    }

    #[qjs(set, rename = "onclick")]
    pub fn set_onclick<'js>(&self, listener: Function<'js>) {
        self.add_event_listener("click".to_string(), listener);
    }

    #[qjs(set, rename = "onload")]
    pub fn set_onload<'js>(&self, listener: Function<'js>) {
        self.add_event_listener("load".to_string(), listener);
    }

    #[qjs(rename = "createDocumentFragment")]
    pub fn create_document_fragment<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let fragment = self::fragment::DocumentFragment::new(
            self.dom.clone(),
            self.mutations.clone(),
            self.stylesheet_dirty.clone(),
            self.primitives.clone(),
        );
        let instance = Class::instance(ctx, fragment)?;
        Ok(instance.into_value())
    }

    #[qjs(rename = "createEvent")]
    pub fn create_event<'js>(&self, ctx: Ctx<'js>, _type_name: String) -> Result<Value<'js>> {
        // Legacy: document.createEvent("HTMLEvents")
        // Just return a basic Event object
        let event = crate::js::bindings::event::Event {
            type_: "event".into(),
            bubbles: true,
            cancelable: true,
            target: None,
            current_target: None,
        };
        let instance = Class::instance(ctx, event)?;
        Ok(instance.into_value())
    }

    #[qjs(rename = "createRange")]
    pub fn create_range<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let range = crate::js::bindings::range::Range::new();
        let instance = Class::instance(ctx, range)?;
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
                primitives: self.primitives.clone(),
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
                primitives: self.primitives.clone(),
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

    #[qjs(get, rename = "images")]
    pub fn images<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let collection = self::collections::HtmlCollection {
            dom: self.dom.clone(),
            selector_fn: std::sync::Arc::new(|dom, idx| {
                if let Some(node) = dom.get_node(idx) {
                    if let AceNodeType::Element(el) = &node.node_type {
                        return el.tag == "img";
                    }
                }
                false
            }),
            mutations: self.mutations.clone(),
            stylesheet_dirty: self.stylesheet_dirty.clone(),
        };
        let instance = Class::instance(ctx, collection)?;
        Ok(instance.into_value())
    }

    #[qjs(get, rename = "links")]
    pub fn links<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let collection = self::collections::HtmlCollection {
            dom: self.dom.clone(),
            selector_fn: std::sync::Arc::new(|dom, idx| {
                if let Some(node) = dom.get_node(idx) {
                    if let AceNodeType::Element(el) = &node.node_type {
                        return el.tag == "a" && el.attributes.contains_key("href");
                    }
                }
                false
            }),
            mutations: self.mutations.clone(),
            stylesheet_dirty: self.stylesheet_dirty.clone(),
        };
        let instance = Class::instance(ctx, collection)?;
        Ok(instance.into_value())
    }

    #[qjs(get, rename = "forms")]
    pub fn forms<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let collection = self::collections::HtmlCollection {
            dom: self.dom.clone(),
            selector_fn: std::sync::Arc::new(|dom, idx| {
                if let Some(node) = dom.get_node(idx) {
                    if let AceNodeType::Element(el) = &node.node_type {
                        return el.tag == "form";
                    }
                }
                false
            }),
            mutations: self.mutations.clone(),
            stylesheet_dirty: self.stylesheet_dirty.clone(),
        };
        let instance = Class::instance(ctx, collection)?;
        Ok(instance.into_value())
    }
    
    #[qjs(get, rename = "scripts")]
    pub fn scripts<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let collection = self::collections::HtmlCollection {
            dom: self.dom.clone(),
            selector_fn: std::sync::Arc::new(|dom, idx| {
                if let Some(node) = dom.get_node(idx) {
                    if let AceNodeType::Element(el) = &node.node_type {
                        return el.tag == "script";
                    }
                }
                false
            }),
            mutations: self.mutations.clone(),
            stylesheet_dirty: self.stylesheet_dirty.clone(),
        };
        let instance = Class::instance(ctx, collection)?;
        Ok(instance.into_value())
    }

    #[qjs(get, rename = "readyState")]
    pub fn ready_state(&self) -> String {
        "complete".to_string() // Simplified for now
    }

    #[qjs(get, rename = "cookie")]
    pub fn get_cookie(&self) -> String {
        self.cookie_storage.lock().unwrap().clone()
    }

    #[qjs(set, rename = "cookie")]
    pub fn set_cookie(&self, cookie: String) {
        let mut storage = self.cookie_storage.lock().unwrap();
        // Naive implementation: just append or replace
        // Real implementation should handle expiration, path, etc.
        if storage.is_empty() {
            *storage = cookie;
        } else {
            *storage = format!("{}; {}", *storage, cookie);
        }
    }

    #[qjs(get, rename = "URL")]
    pub fn url(&self) -> String {
        self.url.clone()
    }

    #[qjs(get, rename = "referrer")]
    pub fn referrer(&self) -> String {
        self.referrer.clone()
    }

    #[qjs(get, rename = "activeElement")]
    pub fn active_element<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let dom = self.dom.lock().unwrap();
        if let Some(idx) = dom.active_element {
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
pub fn register(rt: &JsRuntime, dom: Arc<Mutex<AceDOM>>, stylesheet: std::sync::Arc<std::sync::Mutex<crate::engine::style::Stylesheet>>, primitives: Arc<Mutex<Vec<crate::engine::ACEPrimitive>>>, url: String, referrer: String) -> Result<()> {
    rt.with_context(|context| {
        context.with(|ctx| {
            // Register classes
            Class::<Element>::define(&ctx.globals())?;
            Class::<crate::js::bindings::element::attributes::NamedNodeMap>::define(&ctx.globals())?;
            Class::<crate::js::bindings::token_list::DomTokenList>::define(&ctx.globals())?;
            Class::<self::collections::HtmlCollection>::define(&ctx.globals())?;
            Class::<self::fragment::DocumentFragment>::define(&ctx.globals())?;
            Class::<crate::js::bindings::style_declaration::CssStyleDeclaration>::define(&ctx.globals())?;
            Class::<crate::js::bindings::computed_style::ComputedCSSStyleDeclaration>::define(&ctx.globals())?;
            Class::<crate::js::bindings::event::Event>::define(&ctx.globals())?;
            Class::<crate::js::bindings::mutation_observer::MutationObserver>::define(&ctx.globals())?;
            Class::<crate::js::bindings::element::rect::DOMRect>::define(&ctx.globals())?;
            Class::<crate::js::bindings::range::Range>::define(&ctx.globals())?;
            Class::<crate::js::bindings::selection::Selection>::define(&ctx.globals())?;
            Class::<crate::js::bindings::parser::DOMParser>::define(&ctx.globals())?;
            Class::<Document>::define(&ctx.globals())?;
            
            // Create instance and set as global 'document'
            let doc_instance = Class::instance(ctx.clone(), Document { 
                dom, 
                stylesheet: stylesheet.clone(),
                mutations: rt.mutations.clone(),
                stylesheet_dirty: rt.stylesheet_dirty.clone(),
                cookie_storage: Arc::new(Mutex::new(String::new())),
                primitives,
                url,
                referrer,
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
