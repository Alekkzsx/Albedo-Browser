use super::*;
use crate::ace::engine::dom::{AceDOM, AceNodeType};
use rquickjs::{prelude::Rest, Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};



#[rquickjs::methods]
impl Element {

    #[qjs(get, rename = "innerHTML")]
    pub fn inner_html(&self) -> String {
        self::props::inner_html(self)
    }

    #[qjs(set, rename = "innerHTML")]
    pub fn set_inner_html(&self, html: String) {
        self::props::set_inner_html(self, html)
    }

    #[qjs(get, rename = "outerHTML")]
    pub fn outer_html(&self) -> String {
        self::props::outer_html(self)
    }

    #[qjs(set, rename = "outerHTML")]
    pub fn set_outer_html(&self, html: String) {
        self::props::set_outer_html(self, html)
    }

    #[qjs(rename = "remove")]
    pub fn remove(&self) {
        self::hierarchy::remove(self)
    }

    #[qjs(rename = "contains")]
    pub fn contains(&self, other: Value<'_>) -> bool {
        self::hierarchy::contains(self, other)
    }

    #[qjs(get, rename = "scrollTop")]
    pub fn scroll_top(&self) -> f32 {
        self::props::scroll_top(self)
    }

    #[qjs(set, rename = "scrollTop")]
    pub fn set_scroll_top(&self, val: f32) {
        self::props::set_scroll_top(self, val)
    }

    #[qjs(get, rename = "scrollLeft")]
    pub fn scroll_left(&self) -> f32 {
        self::props::scroll_left(self)
    }

    #[qjs(set, rename = "scrollLeft")]
    pub fn set_scroll_left(&self, val: f32) {
        self::props::set_scroll_left(self, val)
    }

    #[qjs(get, rename = "scrollWidth")]
    pub fn scroll_width(&self) -> f32 {
        self::props::scroll_width(self)
    }

    #[qjs(get, rename = "scrollHeight")]
    pub fn scroll_height(&self) -> f32 {
        self::props::scroll_height(self)
    }

    #[qjs(get, rename = "clientWidth")]
    pub fn client_width(&self) -> f32 {
        self::props::client_width(self)
    }

    #[qjs(get, rename = "clientHeight")]
    pub fn client_height(&self) -> f32 {
        self::props::client_height(self)
    }

    #[qjs(get, rename = "clientTop")]
    pub fn client_top(&self) -> f32 {
        self::props::client_top(self)
    }

    #[qjs(get, rename = "clientLeft")]
    pub fn client_left(&self) -> f32 {
        self::props::client_left(self)
    }

    #[qjs(get, rename = "offsetWidth")]
    pub fn offset_width(&self) -> f32 {
        self::props::offset_width(self)
    }

    #[qjs(get, rename = "offsetHeight")]
    pub fn offset_height(&self) -> f32 {
        self::props::offset_height(self)
    }

    #[qjs(get, rename = "offsetTop")]
    pub fn offset_top(&self) -> f32 {
        self::props::offset_top(self)
    }

    #[qjs(get, rename = "offsetLeft")]
    pub fn offset_left(&self) -> f32 {
        self::props::offset_left(self)
    }
}
