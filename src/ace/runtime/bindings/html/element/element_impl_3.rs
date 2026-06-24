use super::*;
use crate::ace::engine::dom::{AceDOM, AceNodeType};
use rquickjs::{prelude::Rest, Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};



#[rquickjs::methods]
impl Element {
    #[qjs(rename = "insertBefore")]
    pub fn insert_before<'js>(
        &self,
        ctx: Ctx<'js>,
        child: Value<'js>,
        ref_child: Value<'js>,
    ) -> Result<Value<'js>> {
        self::hierarchy::insert_before(self, ctx, child, ref_child)
    }

    #[qjs(rename = "append")]
    pub fn append<'js>(&self, ctx: Ctx<'js>, nodes: Rest<Value<'js>>) -> Result<()> {
        self::hierarchy::append(self, ctx, nodes)
    }

    #[qjs(rename = "prepend")]
    pub fn prepend<'js>(&self, ctx: Ctx<'js>, nodes: Rest<Value<'js>>) -> Result<()> {
        self::hierarchy::prepend(self, ctx, nodes)
    }

    #[qjs(get, rename = "parentNode")]
    pub fn parent_node<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::hierarchy::parent_node(self, ctx)
    }

    #[qjs(get, rename = "parentElement")]
    pub fn parent_element<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::hierarchy::parent_element(self, ctx)
    }

    #[qjs(get, rename = "firstChild")]
    pub fn first_child<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::hierarchy::first_child(self, ctx)
    }

    #[qjs(get, rename = "lastChild")]
    pub fn last_child<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::hierarchy::last_child(self, ctx)
    }

    #[qjs(get, rename = "nextSibling")]
    pub fn next_sibling<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::hierarchy::next_sibling(self, ctx)
    }

    #[qjs(get, rename = "previousSibling")]
    pub fn previous_sibling<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::hierarchy::previous_sibling(self, ctx)
    }

    #[qjs(get, rename = "children")]
    pub fn children<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::hierarchy::children(self, ctx)
    }

    #[qjs(get, rename = "firstElementChild")]
    pub fn first_element_child<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::hierarchy::first_element_child(self, ctx)
    }

    #[qjs(get, rename = "lastElementChild")]
    pub fn last_element_child<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::hierarchy::last_element_child(self, ctx)
    }

    #[qjs(get, rename = "nextElementSibling")]
    pub fn next_element_sibling<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::hierarchy::next_element_sibling(self, ctx)
    }

    #[qjs(get, rename = "previousElementSibling")]
    pub fn previous_element_sibling<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::hierarchy::previous_element_sibling(self, ctx)
    }

    #[qjs(get, rename = "childElementCount")]
    pub fn child_element_count(&self) -> usize {
        self::hierarchy::child_element_count(self)
    }

    #[qjs(rename = "isSameNode")]
    pub fn is_same_node(&self, other: Value<'_>) -> bool {
        self::hierarchy::is_same_node(self, other)
    }

    #[qjs(rename = "matches")]
    pub fn matches(&self, selector: String) -> bool {
        self::query::matches(self, selector)
    }

    #[qjs(rename = "closest")]
    pub fn closest<'js>(&self, ctx: Ctx<'js>, selector: String) -> Result<Value<'js>> {
        self::query::closest(self, ctx, selector)
    }

    #[qjs(rename = "getBoundingClientRect")]
    pub fn get_bounding_client_rect<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::rect::get_bounding_client_rect(self, ctx)
    }
}
