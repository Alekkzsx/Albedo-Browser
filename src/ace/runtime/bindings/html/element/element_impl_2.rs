use super::*;
use crate::ace::engine::dom::{AceDOM, AceNodeType};
use rquickjs::{prelude::Rest, Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};



#[rquickjs::methods]
impl Element {
    #[qjs(get, rename = "onchange")]
    pub fn onchange_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onchange")]
    pub fn onchange_setter<'js>(&self, listener: Function<'js>) {
        self.add_event_listener("change".to_string(), listener);
    }

    #[qjs(get, rename = "onsubmit")]
    pub fn onsubmit_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onsubmit")]
    pub fn onsubmit_setter<'js>(&self, listener: Function<'js>) {
        self.add_event_listener("submit".to_string(), listener);
    }

    #[qjs(rename = "attachShadow")]
    pub fn attach_shadow<'js>(
        &self,
        ctx: Ctx<'js>,
        _options: rquickjs::Object<'js>,
    ) -> Result<Value<'js>> {
        self::shadow::attach_shadow(self, ctx)
    }

    #[qjs(get, rename = "shadowRoot")]
    pub fn shadow_root<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::shadow::get_shadow_root(self, ctx)
    }

    #[qjs(rename = "scrollIntoView")]
    pub fn scroll_into_view(&self) {
        if let Ok(mut ps) = self.pending_scroll.lock() {
            *ps = Some(self.index);
        }
    }

    #[qjs(rename = "insertAdjacentHTML")]
    pub fn insert_adjacent_html<'js>(
        &self,
        ctx: Ctx<'js>,
        position: String,
        html: String,
    ) -> Result<()> {
        self::hierarchy::insert_adjacent_html(self, ctx, position, html)
    }

    #[qjs(rename = "insertAdjacentElement")]
    pub fn insert_adjacent_element<'js>(
        &self,
        ctx: Ctx<'js>,
        position: String,
        element: Class<'js, Element>,
    ) -> Result<Class<'js, Element>> {
        self::hierarchy::insert_adjacent_element(self, ctx, position, element)
    }

    #[qjs(get, rename = "style")]
    pub fn style<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::style::style(self, ctx)
    }

    #[qjs(get, rename = "classList")]
    pub fn class_list<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::style::class_list(self, ctx)
    }

    #[qjs(get, rename = "dataset")]
    pub fn dataset<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::dataset::dataset(self, ctx)
    }

    #[qjs(get, rename = "tagName")]
    pub fn tag_name(&self) -> String {
        self::props::tag_name(self)
    }

    #[qjs(get, rename = "attributes")]
    pub fn attributes<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::attributes::attributes(self, ctx)
    }

    #[qjs(rename = "appendChild")]
    pub fn append_child<'js>(&self, ctx: Ctx<'js>, child: Value<'js>) -> Result<Value<'js>> {
        self::hierarchy::append_child(self, ctx, child)
    }

    #[qjs(rename = "removeChild")]
    pub fn remove_child<'js>(
        &self,
        ctx: Ctx<'js>,
        child: Class<'js, Element>,
    ) -> Result<Class<'js, Element>> {
        self::hierarchy::remove_child(self, ctx, child)
    }

}
