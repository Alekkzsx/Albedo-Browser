use crate::ace::engine::dom::{AceDOM, AceNodeType};
use rquickjs::{prelude::Rest, Class, Ctx, Function, Result, Value};
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
    #[qjs(skip_trace)]
    pub primitives: Arc<Mutex<Vec<crate::ace::engine::ACEPrimitive>>>,
    #[qjs(skip_trace)]
    pub canvas_contexts:
        Arc<Mutex<std::collections::HashMap<usize, crate::ace::engine::graphics::canvas2d::Canvas2D>>>,
    #[qjs(skip_trace)]
    pub pending_scroll: Arc<Mutex<Option<usize>>>,
    #[qjs(skip_trace)]
    pub element_geometry:
        Arc<Mutex<std::collections::HashMap<usize, crate::ace::engine::ElementGeometry>>>,
    #[qjs(skip_trace)]
    pub element_scroll: Arc<Mutex<std::collections::HashMap<usize, (f32, f32)>>>,
}

pub mod attributes;
pub mod canvas;
pub mod canvas_context;
pub mod dataset;
pub mod events;
pub mod hierarchy;
pub mod props;
pub mod query;
pub mod rect;
pub mod shadow;
pub mod style;

pub(crate) use self::Element as ElementType;
pub(crate) fn mark_mutation(el: &ElementType) {
    el.mark_mutation();
}

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

    #[qjs(get, rename = "origin")]
    pub fn origin(&self) -> String {
        let href = self::props::href(self);
        if let Ok(u) = crate::ace::url::parse(&href, None) {
            return u.origin();
        }
        "".to_string()
    }

    #[qjs(get, rename = "pathname")]
    pub fn pathname(&self) -> String {
        let href = self::props::href(self);
        if let Ok(u) = crate::ace::url::parse(&href, None) {
            return u.path();
        }
        "".to_string()
    }

    #[qjs(get, rename = "width")]
    pub fn width(&self) -> i32 {
        self::props::width(self)
    }

    #[qjs(set, rename = "width")]
    pub fn set_width(&self, val: i32) {
        self::props::set_width(self, val)
    }

    #[qjs(get, rename = "height")]
    pub fn height(&self) -> i32 {
        self::props::height(self)
    }

    #[qjs(set, rename = "height")]
    pub fn set_height(&self, val: i32) {
        self::props::set_height(self, val)
    }

    #[qjs(rename = "getContext")]
    pub fn get_context<'js>(&self, ctx: Ctx<'js>, type_: String) -> Result<Value<'js>> {
        self::canvas::get_context(self, ctx, type_)
    }

    // ─── HTMLDialogElement API ───────────────────────────────────────

    /// dialog.show() — abre o dialog como non-modal
    #[qjs(rename = "show")]
    pub fn dialog_show(&self) {
        let is_dialog = {
            let dom = self.dom.lock().unwrap();
            if let Some(node) = dom.get_node(self.index) {
                matches!(&node.node_type, AceNodeType::Element(el) if el.tag == "dialog")
            } else {
                false
            }
        };
        if !is_dialog {
            return;
        }

        // Se já está aberto, no-op
        if self.has_attribute("open".into()) {
            return;
        }

        self.set_attribute("open".into(), String::new());
        self.mark_mutation();
    }

    /// dialog.showModal() — abre como modal com backdrop
    #[qjs(rename = "showModal")]
    pub fn dialog_show_modal(&self) {
        let is_dialog = {
            let dom = self.dom.lock().unwrap();
            if let Some(node) = dom.get_node(self.index) {
                matches!(&node.node_type, AceNodeType::Element(el) if el.tag == "dialog")
            } else {
                false
            }
        };
        if !is_dialog {
            return;
        }

        // Se já está aberto, no-op (spec diz InvalidStateError, mas sem throw por agora)
        if self.has_attribute("open".into()) {
            return;
        }

        self.set_attribute("open".into(), String::new());
        self.set_attribute("data-ace-modal".into(), String::new());
        self.mark_mutation();
    }

    /// dialog.close(returnValue?) — fecha o dialog, dispara evento "close"
    #[qjs(rename = "close")]
    pub fn dialog_close(&self, return_value: Option<String>) {
        let is_dialog = {
            let dom = self.dom.lock().unwrap();
            if let Some(node) = dom.get_node(self.index) {
                matches!(&node.node_type, AceNodeType::Element(el) if el.tag == "dialog")
            } else {
                false
            }
        };
        if !is_dialog {
            return;
        }

        // Se não está aberto, no-op
        if !self.has_attribute("open".into()) {
            return;
        }

        // Armazenar returnValue se fornecido
        if let Some(rv) = return_value {
            self.set_attribute("data-return-value".into(), rv);
        }

        self.remove_attribute("open".into());
        self.remove_attribute("data-ace-modal".into());
        self.mark_mutation();

        // Disparar evento "close" no nó (via DOM mutation — o pulse tratará)
        // Nota: Para disparar imediatamente, seria necessário acesso ao JS runtime aqui.
        // A marca de mutation garante que o layout recompute.
    }

    /// dialog.open (getter) — retorna true se o atributo "open" está presente
    #[qjs(get, rename = "open")]
    pub fn dialog_open_get(&self) -> bool {
        self.has_attribute("open".into())
    }

    /// dialog.returnValue (getter)
    #[qjs(get, rename = "returnValue")]
    pub fn dialog_return_value_get(&self) -> String {
        self.get_attribute("data-return-value".into())
            .unwrap_or_default()
    }

    /// dialog.returnValue (setter)
    #[qjs(set, rename = "returnValue")]
    pub fn dialog_return_value_set(&self, val: String) {
        self.set_attribute("data-return-value".into(), val);
    }
}
