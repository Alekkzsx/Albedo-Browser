use rquickjs::{Ctx, Class, Result, Value, Function};
use crate::js::bindings::token_list::DomTokenList;
use crate::engine::dom::{AceDOM, AceNodeType};
use std::sync::{Arc, Mutex};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Element {
    #[qjs(skip_trace)]
    pub dom: Arc<Mutex<AceDOM>>,
    #[qjs(skip_trace)]
    pub index: usize,
    #[qjs(skip_trace)]
    pub mutations: Arc<Mutex<bool>>,
    #[qjs(skip_trace)]
    pub stylesheet_dirty: Arc<Mutex<bool>>,
}

pub mod query;
pub mod events;
pub mod hierarchy;
pub mod props;
pub mod style;

pub(crate) use self::Element as ElementType;
pub(crate) fn mark_mutation(el: &ElementType) {
    el.mark_mutation();
}

#[rquickjs::methods]
impl Element {
    #[qjs(get, rename = "node_idx")]
    pub fn get_node_idx(&self) -> usize {
        self.index
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
        self::events::dispatch_event(self, ctx, event)
    }

    #[qjs(get, rename = "style")]
    pub fn style<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::style::style(self, ctx)
    }

    #[qjs(get, rename = "classList")]
    pub fn class_list<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::style::class_list(self, ctx)
    }

    #[qjs(get, rename = "tagName")]
    pub fn tag_name(&self) -> String {
        self::props::tag_name(self)
    }

    #[qjs(get, rename = "textContent")]
    pub fn text_content(&self) -> String {
        self::props::text_content(self)
    }
    
    #[qjs(set, rename = "textContent")]
    pub fn set_text_content(&self, text: String) {
        self::props::set_text_content(self, text)
    }

    #[qjs(rename = "getAttribute")]
    pub fn get_attribute(&self, name: String) -> Option<String> {
        self::props::get_attribute(self, name)
    }
    
    #[qjs(rename = "setAttribute")]
    pub fn set_attribute(&self, name: String, value: String) {
        self::props::set_attribute(self, name, value)
    }

    #[qjs(rename = "attachShadow")]
    pub fn attach_shadow<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::props::attach_shadow(self, ctx)
    }

    #[qjs(rename = "hasAttribute")]
    pub fn has_attribute(&self, name: String) -> bool {
        self::props::has_attribute(self, name)
    }

    #[qjs(rename = "removeAttribute")]
    pub fn remove_attribute(&self, name: String) {
        self::props::remove_attribute(self, name)
    }
    
    #[qjs(get, rename = "innerHTML")]
    pub fn inner_html(&self) -> String {
        self::props::inner_html(self)
    }
    
    #[qjs(set, rename = "innerHTML")]
    pub fn set_inner_html(&self, html: String) {
        self::props::set_inner_html(self, html)
    }

    #[qjs(rename = "appendChild")]
    pub fn append_child<'js>(&self, ctx: Ctx<'js>, child: Class<'js, Element>) -> Result<Class<'js, Element>> {
        self::hierarchy::append_child(self, ctx, child)
    }

    #[qjs(rename = "removeChild")]
    pub fn remove_child<'js>(&self, ctx: Ctx<'js>, child: Class<'js, Element>) -> Result<Class<'js, Element>> {
        self::hierarchy::remove_child(self, ctx, child)
    }

    #[qjs(rename = "insertBefore")]
    pub fn insert_before<'js>(&self, ctx: Ctx<'js>, child: Class<'js, Element>, ref_child: Value<'js>) -> Result<Class<'js, Element>> {
        self::hierarchy::insert_before(self, ctx, child, ref_child)
    }

    #[qjs(rename = "querySelector")]
    pub fn query_selector<'js>(&self, ctx: Ctx<'js>, selector: String) -> Result<Value<'js>> {
        self::query::query_selector(self, ctx, selector)
    }

    #[qjs(rename = "querySelectorAll")]
    pub fn query_selector_all<'js>(&self, ctx: Ctx<'js>, selector: String) -> Result<Value<'js>> {
        self::query::query_selector_all(self, ctx, selector)
    }

    #[qjs(get, rename = "children")]
    pub fn children<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::hierarchy::children(self, ctx)
    }

    #[qjs(get, rename = "parentElement")]
    pub fn parent_element<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::hierarchy::parent_element(self, ctx)
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
}
