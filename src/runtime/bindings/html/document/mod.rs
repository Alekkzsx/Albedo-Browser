use super::element::Element;
use crate::engine::dom::{AceDOM, AceNode, AceNodeType};
use crate::engine::style::Stylesheet;
use crate::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};

pub mod collections;
pub mod events;
pub mod fragment;
#[cfg(test)]
mod observer_tests;
pub mod query;
#[cfg(test)]
mod tests;

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
    pub primitives: Arc<Mutex<Vec<crate::engine::ACEPrimitive>>>,
    #[qjs(skip_trace)]
    pub canvas_contexts:
        Arc<Mutex<std::collections::HashMap<usize, crate::engine::graphics::canvas2d::Canvas2D>>>,
    #[qjs(skip_trace)]
    pub pending_scroll: Arc<Mutex<Option<usize>>>,
    #[qjs(skip_trace)]
    pub cookie_storage: Arc<Mutex<String>>,
    #[qjs(skip_trace)]
    pub resource_manager: Option<crate::network::resources::ResourceManager>,
    pub url: String,
    pub referrer: String,
    #[qjs(skip_trace)]
    pub element_geometry:
        Arc<Mutex<std::collections::HashMap<usize, crate::engine::ElementGeometry>>>,
    #[qjs(skip_trace)]
    pub element_scroll: Arc<Mutex<std::collections::HashMap<usize, (f32, f32)>>>,
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
    pub fn dispatch_event<'js>(&self, _ctx: Ctx<'js>, event: Value<'js>) -> bool {
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
        if let Some(ref rm) = self.resource_manager {
            return rm.cookie_jar.lock().unwrap().get_cookies_for_url(&self.url);
        }
        "".to_string()
    }

    #[qjs(set, rename = "cookie")]
    pub fn set_cookie(&self, val: String) {
        if let Some(ref rm) = self.resource_manager {
            rm.cookie_jar.lock().unwrap().set_cookie(&self.url, &val);
        }
    }

    #[qjs(get, rename = "title")]
    pub fn title(&self) -> String {
        let dom = self.dom.lock().unwrap();
        if let Some(head_idx) = dom.head {
            if let Some(head_node) = dom.get_node(head_idx) {
                for &child_idx in &head_node.children {
                    if let Some(child) = dom.get_node(child_idx) {
                        if let AceNodeType::Element(el) = &child.node_type {
                            if el.tag == "title" {
                                return dom.serialize_subtree_text(child_idx);
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
    }

    #[qjs(rename = "createElement")]
    pub fn create_element<'js>(&self, ctx: Ctx<'js>, tag: String) -> Result<Value<'js>> {
        let idx = if let Ok(mut dom) = self.dom.lock() {
            let idx = dom.nodes.len();
            dom.nodes.push(AceNode {
                node_type: AceNodeType::Element(crate::engine::dom::AceElement {
                    tag,
                    namespace: crate::ace::html::Namespace::Html,
                    attributes: std::collections::HashMap::new(),
                }),
                parent: None,
                children: Vec::new(),
                prev_sibling: None,
                next_sibling: None,
                shadow_root: None,
                dirty: crate::engine::dom::NodeDirtyFlags::LAYOUT
                    | crate::engine::dom::NodeDirtyFlags::STYLE,
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
            canvas_contexts: self.canvas_contexts.clone(),
            pending_scroll: self.pending_scroll.clone(),
            element_geometry: self.element_geometry.clone(),
            element_scroll: self.element_scroll.clone(),
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
                dirty: crate::engine::dom::NodeDirtyFlags::LAYOUT
                    | crate::engine::dom::NodeDirtyFlags::STYLE,
            });
            idx
        } else {
            return Ok(Value::new_null(ctx));
        };

        let frag = crate::runtime::bindings::html::document::fragment::DocumentFragment {
            dom: self.dom.clone(),
            index: idx,
            mutations: self.mutations.clone(),
            stylesheet_dirty: self.stylesheet_dirty.clone(),
            primitives: self.primitives.clone(),
            canvas_contexts: self.canvas_contexts.clone(),
            pending_scroll: self.pending_scroll.clone(),
            element_geometry: self.element_geometry.clone(),
            element_scroll: self.element_scroll.clone(),
        };

        let instance = Class::instance(ctx, frag)?;
        Ok(instance.into_value())
    }

    #[qjs(rename = "createTextNode")]
    pub fn create_text_node<'js>(&self, ctx: Ctx<'js>, text: String) -> Result<Value<'js>> {
        let idx = if let Ok(mut dom) = self.dom.lock() {
            let idx = dom.nodes.len();
            dom.nodes.push(AceNode {
                node_type: AceNodeType::Text(std::sync::Arc::from(text)),
                parent: None,
                children: Vec::new(),
                prev_sibling: None,
                next_sibling: None,
                shadow_root: None,
                dirty: crate::engine::dom::NodeDirtyFlags::LAYOUT
                    | crate::engine::dom::NodeDirtyFlags::STYLE,
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
            canvas_contexts: self.canvas_contexts.clone(),
            pending_scroll: self.pending_scroll.clone(),
            element_geometry: self.element_geometry.clone(),
            element_scroll: self.element_scroll.clone(),
        };

        let instance = Class::instance(ctx, element)?;
        Ok(instance.into_value())
    }

    #[qjs(rename = "createComment")]
    pub fn create_comment<'js>(&self, ctx: Ctx<'js>, data: String) -> Result<Value<'js>> {
        let idx = if let Ok(mut dom) = self.dom.lock() {
            let idx = dom.nodes.len();
            dom.nodes.push(AceNode {
                node_type: AceNodeType::Comment(std::sync::Arc::from(data)),
                parent: None,
                children: Vec::new(),
                prev_sibling: None,
                next_sibling: None,
                shadow_root: None,
                dirty: crate::engine::dom::NodeDirtyFlags::LAYOUT
                    | crate::engine::dom::NodeDirtyFlags::STYLE,
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
            canvas_contexts: self.canvas_contexts.clone(),
            pending_scroll: self.pending_scroll.clone(),
            element_geometry: self.element_geometry.clone(),
            element_scroll: self.element_scroll.clone(),
        };

        let instance = Class::instance(ctx, element)?;
        Ok(instance.into_value())
    }

    #[qjs(get, rename = "onclick")]
    pub fn onclick_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onclick")]
    pub fn onclick_setter<'js>(&self, listener: Function<'js>) {
        self.add_event_listener("click".to_string(), listener);
    }

    #[qjs(get, rename = "onload")]
    pub fn onload_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onload")]
    pub fn onload_setter<'js>(&self, listener: Function<'js>) {
        self.add_event_listener("load".to_string(), listener);
    }

    #[qjs(rename = "createEvent")]
    pub fn create_event<'js>(&self, ctx: Ctx<'js>, _type_name: String) -> Result<Value<'js>> {
        let event = super::event::Event {
            type_: "event".into(),
            bubbles: true,
            cancelable: true,
            target: None,
            current_target: None,
            cancel_bubble: false,
        };
        let instance = Class::instance(ctx, event)?;
        Ok(instance.into_value())
    }

    #[qjs(rename = "createRange")]
    pub fn create_range<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let range = super::range::Range::new();
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
                canvas_contexts: self.canvas_contexts.clone(),
                pending_scroll: self.pending_scroll.clone(),
                element_geometry: self.element_geometry.clone(),
                element_scroll: self.element_scroll.clone(),
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
                canvas_contexts: self.canvas_contexts.clone(),
                pending_scroll: self.pending_scroll.clone(),
                element_geometry: self.element_geometry.clone(),
                element_scroll: self.element_scroll.clone(),
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
            primitives: self.primitives.clone(),
            canvas_contexts: self.canvas_contexts.clone(),
            pending_scroll: self.pending_scroll.clone(),
            element_geometry: self.element_geometry.clone(),
            element_scroll: self.element_scroll.clone(),
        };
        let instance = Class::instance(ctx, element)?;
        Ok(instance.into_value())
    }

    #[qjs(get, rename = "readyState")]
    pub fn ready_state(&self) -> String {
        "complete".to_string()
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
                primitives: self.primitives.clone(),
                canvas_contexts: self.canvas_contexts.clone(),
                pending_scroll: self.pending_scroll.clone(),
                element_geometry: self.element_geometry.clone(),
                element_scroll: self.element_scroll.clone(),
            };
            let instance = Class::instance(ctx, element)?;
            return Ok(instance.into_value());
        }
        Ok(Value::new_null(ctx))
    }
}

pub fn register(
    rt: &JsRuntime,
    dom: Arc<Mutex<AceDOM>>,
    stylesheet: std::sync::Arc<std::sync::Mutex<crate::engine::style::Stylesheet>>,
    primitives: Arc<Mutex<Vec<crate::engine::ACEPrimitive>>>,
    canvas_contexts: Arc<
        Mutex<std::collections::HashMap<usize, crate::engine::graphics::canvas2d::Canvas2D>>,
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

            let cookies = if let Some(rm) = resource_manager.as_ref() {
                rm.cookie_jar.lock().unwrap().get_cookies_for_url(&url)
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
                    url,
                    referrer,
                },
            )?;
            global.set("document", doc_instance)?;

            Ok(())
        })
    })
}
