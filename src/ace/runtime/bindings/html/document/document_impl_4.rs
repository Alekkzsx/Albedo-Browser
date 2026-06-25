use super::*;
use super::element::Element;
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use crate::ace::engine::style::Stylesheet;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};



#[rquickjs::methods]
impl Document {
    #[qjs(get, rename = "documentElement")]
    pub fn document_element<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let dom = self.dom.lock().unwrap_or_else(|e| e.into_inner());
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
        self.ready_state_ptr.lock().unwrap_or_else(|e| e.into_inner()).clone()
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
        let dom = self.dom.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(node_index) = dom.active_element {
            drop(dom);
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
            return Ok(instance.into_value());
        }
        Ok(Value::new_null(ctx))
    }
}
