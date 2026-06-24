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
        let dom = self.dom.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(body_idx) = dom.body {
            let node_index = body_idx;
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

    #[qjs(get, rename = "head")]
    pub fn head<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let dom = self.dom.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(head_idx) = dom.head {
            let node_index = head_idx;
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
