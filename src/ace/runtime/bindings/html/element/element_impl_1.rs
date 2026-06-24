use super::*;
use crate::ace::engine::dom::{AceDOM, AceNodeType};
use rquickjs::{prelude::Rest, Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};



#[rquickjs::methods]
impl Element {
    #[qjs(get, rename = "textContent")]
    pub fn get_text_content(&self) -> String {
        self::props::text_content(self)
    }

    #[qjs(set, rename = "textContent")]
    pub fn set_text_content(&self, text: String) {
        self::props::set_text_content(self, text)
    }

    #[qjs(get, rename = "node_idx")]
    pub fn get_node_idx(&self) -> usize {
        self.index
    }

    #[qjs(rename = "hasAttribute")]
    pub fn has_attribute(&self, name: String) -> bool {
        self::props::has_attribute(self, name)
    }

    #[qjs(rename = "toggleAttribute")]
    pub fn toggle_attribute(&self, name: String, force: Option<bool>) -> bool {
        let exists = self.has_attribute(name.clone());
        let should_exist = force.unwrap_or(!exists);

        if should_exist {
            self.set_attribute(name, "".to_string());
            true
        } else {
            self.remove_attribute(name);
            false
        }
    }

    #[qjs(rename = "getAttributeNames")]
    pub fn get_attribute_names<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        // Correcting to use a wrapper if props doesn't have _js version
        let names = self::props::get_attribute_names(self);
        let array = rquickjs::Array::new(ctx)?;
        for (i, name) in names.into_iter().enumerate() {
            array.set(i, name)?;
        }
        Ok(array.into_value())
    }

    pub(crate) fn mark_mutation(&self) {
        if let Ok(mut m) = self.mutations.lock() {
            *m = true;
        }

        // Check if style tag
        if let Ok(dom) = self.dom.lock() {
            if let Some(node) = dom.get_node(self.index) {
                if let AceNodeType::Element(el) = &node.node_type {
                    if el.tag == "style" {
                        if let Ok(mut sd) = self.stylesheet_dirty.lock() {
                            *sd = true;
                        }
                    }
                }
            }
        }
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
    pub fn dispatch_event<'js>(&self, ctx: Ctx<'js>, event: Value<'js>) -> bool {
        self::events::dispatch_event(self, &ctx, event)
    }

    // Event Handler Setters/Getters
    #[qjs(get, rename = "onclick")]
    pub fn onclick_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onclick")]
    pub fn onclick_setter<'js>(&self, listener: Function<'js>) {
        self.add_event_listener("click".to_string(), listener);
    }

    #[qjs(get, rename = "oninput")]
    pub fn oninput_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "oninput")]
    pub fn oninput_setter<'js>(&self, listener: Function<'js>) {
        self.add_event_listener("input".to_string(), listener);
    }

}
