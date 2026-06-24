use super::*;
use crate::ace::engine::dom::{AceDOM, AceNodeType};
use rquickjs::{prelude::Rest, Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};



#[rquickjs::methods]
impl Element {

    #[qjs(get, rename = "innerText")]
    pub fn inner_text(&self) -> String {
        self::props::inner_text(self)
    }

    #[qjs(rename = "getAttribute")]
    pub fn get_attribute(&self, name: String) -> Option<String> {
        self::props::get_attribute(self, name)
    }

    #[qjs(rename = "setAttribute")]
    pub fn set_attribute(&self, name: String, value: String) {
        self::props::set_attribute(self, name, value)
    }

    #[qjs(rename = "removeAttribute")]
    pub fn remove_attribute(&self, name: String) {
        self::props::remove_attribute(self, name)
    }

    #[qjs(get, rename = "id")]
    pub fn id(&self) -> String {
        self::props::id(self)
    }

    #[qjs(set, rename = "id")]
    pub fn set_id(&self, val: String) {
        self::props::set_id(self, val)
    }

    #[qjs(get, rename = "className")]
    pub fn class_name(&self) -> String {
        self::props::class_name(self)
    }

    #[qjs(set, rename = "className")]
    pub fn set_class_name(&self, val: String) {
        self::props::set_class_name(self, val)
    }

    #[qjs(get, rename = "name")]
    pub fn name(&self) -> String {
        self::props::name(self)
    }

    #[qjs(set, rename = "name")]
    pub fn set_name(&self, val: String) {
        self::props::set_name(self, val)
    }

    #[qjs(get, rename = "title")]
    pub fn title(&self) -> String {
        self::props::title_prop(self)
    }

    #[qjs(set, rename = "title")]
    pub fn set_title(&self, val: String) {
        self::props::set_title_prop(self, val)
    }

    #[qjs(get, rename = "src")]
    pub fn src(&self) -> String {
        self::props::src(self)
    }

    #[qjs(set, rename = "src")]
    pub fn set_src(&self, val: String) {
        self::props::set_src(self, val)
    }

    #[qjs(get, rename = "href")]
    pub fn href(&self) -> String {
        self::props::href(self)
    }

    #[qjs(set, rename = "href")]
    pub fn set_href(&self, val: String) {
        self::props::set_href(self, val)
    }

    #[qjs(get, rename = "value")]
    pub fn value(&self) -> String {
        self::props::value(self)
    }

    #[qjs(set, rename = "value")]
    pub fn set_value(&self, val: String) {
        self::props::set_value(self, val)
    }

    #[qjs(get, rename = "checked")]
    pub fn checked(&self) -> bool {
        self::props::checked(self)
    }

    #[qjs(set, rename = "checked")]
    pub fn set_checked(&self, val: bool) {
        self::props::set_checked(self, val)
    }
}
