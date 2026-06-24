use super::*;
use super::element::Element;
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use crate::ace::engine::style::Stylesheet;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};

pub mod collections;
pub mod events;
pub mod fragment;
pub mod query;



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
            return rm.cookie_jar.lock().unwrap_or_else(|e| e.into_inner()).get_cookies_for_url(&self.url);
        }
        "".to_string()
    }

    #[qjs(set, rename = "cookie")]
    pub fn set_cookie(&self, val: String) {
        if let Some(ref rm) = self.resource_manager {
            rm.cookie_jar.lock().unwrap_or_else(|e| e.into_inner()).set_cookie(&self.url, &val);
        }
    }

    #[qjs(get, rename = "title")]
    pub fn title(&self) -> String {
        let dom = self.dom.lock().unwrap_or_else(|e| e.into_inner());
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
        tracing::debug!(title = %title, "Document title set");
    }

    #[qjs(rename = "createElement")]
    pub fn create_element<'js>(&self, ctx: Ctx<'js>, tag: String) -> Result<Value<'js>> {
        let node_index = if let Ok(mut dom) = self.dom.lock() {
            let node_index = dom.nodes.len();
            dom.nodes.push(AceNode {
                node_type: AceNodeType::Element(crate::ace::engine::dom::AceElement {
                    tag,
                    namespace: crate::ace::html::Namespace::Html,
                    attributes: std::collections::HashMap::new(),
                }),
                parent: None,
                children: Vec::new(),
                prev_sibling: None,
                next_sibling: None,
                shadow_root: None,
                dirty: crate::ace::engine::dom::NodeDirtyFlags::LAYOUT
                    | crate::ace::engine::dom::NodeDirtyFlags::STYLE,
            });
            node_index
        } else {
            return Ok(Value::new_null(ctx));
        };

        let element = Element {
            dom: self.dom.clone(),
            index: node_index,
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
}
