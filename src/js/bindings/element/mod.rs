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
    #[qjs(skip_trace)]
    pub primitives: Arc<Mutex<Vec<crate::engine::ACEPrimitive>>>,
}

pub mod hierarchy;
pub mod props;
pub mod style;
pub mod attributes;
pub mod rect;
pub mod shadow;

pub(crate) use self::Element as ElementType;

pub(crate) use self::Element as ElementType;
pub(crate) fn mark_mutation(el: &ElementType) {
    el.mark_mutation();
}

#[rquickjs::methods]
impl Element {
    #[qjs(get, rename = "textContent")]
    pub fn get_text_content(&self) -> String {
        self::hierarchy::get_text_content(self)
    }

    #[qjs(set, rename = "textContent")]
    pub fn set_text_content(&self, text: String) {
        self::hierarchy::set_text_content(self, text)
    }

    #[qjs(get, rename = "node_idx")]
    pub fn get_node_idx(&self) -> usize {
        self.index
    }

    #[qjs(rename = "hasAttribute")]
    pub fn has_attribute(&self, name: String) -> bool {
        if let Ok(dom) = self.dom.lock() {
            if let Some(node) = dom.get_node(self.index) {
                if let AceNodeType::Element(el) = &node.node_type {
                    return el.attributes.contains_key(&name);
                }
            }
        }
        false
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
        let arr = rquickjs::Array::new(ctx.clone())?;
        if let Ok(dom) = self.dom.lock() {
            if let Some(node) = dom.get_node(self.index) {
                if let AceNodeType::Element(el) = &node.node_type {
                    for (i, name) in el.attributes.keys().enumerate() {
                        arr.set(i, name.clone())?;
                    }
                }
            }
        }
        Ok(arr.into_value())
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

    #[qjs(set, rename = "onclick")]
    pub fn set_onclick<'js>(&self, listener: Function<'js>) {
        self.add_event_listener("click".to_string(), listener);
    }

    #[qjs(set, rename = "oninput")]
    pub fn set_oninput<'js>(&self, listener: Function<'js>) {
        self.add_event_listener("input".to_string(), listener);
    }

    #[qjs(set, rename = "onchange")]
    pub fn set_onchange<'js>(&self, listener: Function<'js>) {
        self.add_event_listener("change".to_string(), listener);
    }

    #[qjs(set, rename = "onsubmit")]
    pub fn set_onsubmit<'js>(&self, listener: Function<'js>) {
        self.add_event_listener("submit".to_string(), listener);
    }

    #[qjs(rename = "attachShadow")]
    pub fn attach_shadow<'js>(&self, ctx: Ctx<'js>, _options: rquickjs::Object<'js>) -> Result<Value<'js>> {
        self::shadow::attach_shadow(self, ctx)
    }

    #[qjs(get, rename = "shadowRoot")]
    pub fn shadow_root<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self::shadow::get_shadow_root(self, ctx)
    }

    #[qjs(rename = "scrollIntoView")]
    pub fn scroll_into_view(&self) {
        // Stub: In Albedo this would interact with the global scroll controller
    }

    pub(crate) fn dispatch_event_internal(&self, type_: String) {
        // Simple internal event dispatch without full JS Event object
        let ptr = self.index;
        let dom = self.dom.clone();
        
        let get_parent = move |p: usize| -> Option<usize> {
            if let Ok(d) = dom.lock() {
                if let Some(node) = d.get_node(p) {
                    return node.parent;
                }
            }
            None
        };

        let event_data = crate::js::bindings::event::Event {
            type_: type_.clone(),
            bubbles: true,
            cancelable: true,
            target: None,
            current_target: None,
        };

        let listeners_chain = EventTargetImpl::dispatch_event_with_bubbling(ptr, &event_data, get_parent);
        
        for (_curr_ptr, listener_static) in listeners_chain {
            // This is tricky because we need a context to call listeners.
            // In a better architecture, we'd have a way to get the current context.
            // For now, this is a stub or we can try to use a thread-local context if available.
            // Since Albedo is mostly single-threaded JS, we might have it.
            println!("Internal dispatch: would call listener for {} on element {}", type_, ptr);
            // NOTE: Full implementation would require a reference to the JsRuntime or Current Context.
        }
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

    #[qjs(get, rename = "attributes")]
    pub fn attributes<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let map = self::attributes::NamedNodeMap {
            dom: self.dom.clone(),
            index: self.index,
        };
        let instance = Class::instance(ctx, map)?;
        Ok(instance.into_value())
    }

    #[qjs(rename = "appendChild")]
    pub fn append_child<'js>(&self, ctx: Ctx<'js>, child: Value<'js>) -> Result<Value<'js>> {
        self::hierarchy::append_child(self, ctx, child)
    }

    #[qjs(rename = "removeChild")]
    pub fn remove_child<'js>(&self, ctx: Ctx<'js>, child: Class<'js, Element>) -> Result<Class<'js, Element>> {
        self::hierarchy::remove_child(self, ctx, child)
    }

    #[qjs(rename = "insertBefore")]
    pub fn insert_before<'js>(&self, ctx: Ctx<'js>, child: Value<'js>, ref_child: Value<'js>) -> Result<Value<'js>> {
        self::hierarchy::insert_before(self, ctx, child, ref_child)
    }

    #[qjs(rename = "append")]
    pub fn append<'js>(&self, ctx: Ctx<'js>, nodes: rquickjs::Rest<Value<'js>>) -> Result<()> {
        self::hierarchy::append(self, ctx, nodes)
    }

    #[qjs(rename = "prepend")]
    pub fn prepend<'js>(&self, ctx: Ctx<'js>, nodes: rquickjs::Rest<Value<'js>>) -> Result<()> {
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

    #[qjs(rename = "getBoundingClientRect")]
    pub fn get_bounding_client_rect<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let primitives = self.primitives.lock().unwrap();
        // Search for the primitive matching this node
        // Often common to have multiple (box + text), we want the box or the first one
        if let Some(prim) = primitives.iter().find(|p| p.node_idx == self.index) {
            let rect = self::rect::DOMRect::new(prim.x, prim.y, prim.width, prim.height);
            let instance = Class::instance(ctx, rect)?;
            return Ok(instance.into_value());
        }
        // Fallback or empty rect
        let rect = self::rect::DOMRect::new(0.0, 0.0, 0.0, 0.0);
        let instance = Class::instance(ctx, rect)?;
        Ok(instance.into_value())
    }

    #[qjs(get, rename = "innerText")]
    pub fn inner_text(&self) -> String {
        let primitives = self.primitives.lock().unwrap();
        let mut text = String::new();
        // Layout-aware text retrieval: only include text that is in the display list
        // and handle node descendants
        
        // Simplified approach: find all text primitives that are descendants of this node
        // Since primitives are in render order, we can collect those belonging to descendants.
        
        // But we need to know who the descendants are.
        // Let's use the DOM to find all descendant indices and then match.
        let mut descendant_indices = std::collections::HashSet::new();
        if let Ok(dom) = self.dom.lock() {
            self.collect_descendants(&dom, self.index, &mut descendant_indices);
        }
        descendant_indices.insert(self.index);

        for prim in primitives.iter() {
            if descendant_indices.contains(&prim.node_idx) && prim.element_type == "text" {
                text.push_str(&prim.text);
                text.push(' '); // Naive spacing
            }
        }
        text.trim().to_string()
    }

    fn collect_descendants(&self, dom: &AceDOM, root: usize, set: &mut std::collections::HashSet<usize>) {
        if let Some(node) = dom.get_node(root) {
            for &child in &node.children {
                set.insert(child);
                self.collect_descendants(dom, child, set);
            }
        }
    }
    pub fn get_attribute(&self, name: String) -> Option<String> {
        self::props::get_attribute(self, name)
    }
    
    #[qjs(rename = "setAttribute")]
    pub fn set_attribute(&self, name: String, value: String) {
        self::props::set_attribute(self, name, value)
    }

    #[qjs(rename = "getAttributeNames")]
    pub fn get_attribute_names(&self) -> Vec<String> {
        self::props::get_attribute_names(self)
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

    #[qjs(rename = "matches")]
    pub fn matches(&self, selector: String) -> bool {
        self::query::matches(self, selector)
    }

    #[qjs(rename = "closest")]
    pub fn closest<'js>(&self, ctx: Ctx<'js>, selector: String) -> Result<Value<'js>> {
        self::query::closest(self, ctx, selector)
    }

    #[qjs(rename = "remove")]
    pub fn remove(&self) {
        self::hierarchy::remove(self)
    }

    #[qjs(rename = "contains")]
    pub fn contains(&self, other: Value) -> bool {
        self::hierarchy::contains(self, other)
    }

    #[qjs(get, rename = "scrollTop")]
    pub fn scroll_top(&self) -> f32 {
        self::props::scroll_top(self)
    }

    #[qjs(get, rename = "scrollLeft")]
    pub fn scroll_left(&self) -> f32 {
        self::props::scroll_left(self)
    }

    #[qjs(get, rename = "scrollWidth")]
    pub fn scroll_width(&self) -> f32 {
        self::props::scroll_width(self)
    }

    #[qjs(get, rename = "scrollHeight")]
    pub fn scroll_height(&self) -> f32 {
        self::props::scroll_height(self)
    }

    // Anchor URL helpers
    #[qjs(get, rename = "origin")]
    pub fn origin(&self) -> String {
        let href = self::props::href(self);
        if let Ok(u) = url::Url::parse(&href) {
            return format!("{}://{}", u.scheme(), u.host_str().unwrap_or(""));
        }
        "".to_string()
    }

    #[qjs(get, rename = "pathname")]
    pub fn pathname(&self) -> String {
        let href = self::props::href(self);
        if let Ok(u) = url::Url::parse(&href) {
        "".to_string()
    }

    #[qjs(get, rename = "textContent")]
    pub fn get_text_content(&self) -> String {
         if let Ok(dom) = self.dom.lock() {
            return dom.serialize_subtree_text(self.index);
        }
        String::new()
    }

    #[qjs(set, rename = "textContent")]
    pub fn set_text_content(&self, text: String) {
        if let Ok(mut dom) = self.dom.lock() {
             // Basic implementation: clear children and add a single text node
             if let Some(node) = dom.nodes.get_mut(self.index) {
                 node.children.clear();
             }
             let text_idx = dom.nodes.len();
             dom.nodes.push(crate::engine::dom::AceNode {
                 node_type: crate::engine::dom::AceNodeType::Text(text),
                 parent: Some(self.index),
                 children: Vec::new(),
                 prev_sibling: None,
                 next_sibling: None,
                 shadow_root: None,
             });
             if let Some(node) = dom.nodes.get_mut(self.index) {
                 node.children.push(text_idx);
             }
        }
        self.mark_mutation();
    }

    #[qjs(rename = "insertAdjacentHTML")]
    pub fn insert_adjacent_html(&self, position: String, html: String) {
        // Stub: This requires a parser pass. For now, we'll handle basic append if position is 'beforeend'
        if position == "beforeend" {
             self.set_inner_html(self.get_inner_html() + &html);
        }
    }

    #[qjs(rename = "insertAdjacentElement")]
    pub fn insert_adjacent_element<'js>(&self, _position: String, element: Value<'js>) -> Value<'js> {
        // Limited implementation for common cases
        element
    }

    // Image helpers
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

    #[qjs(get, rename = "naturalWidth")]
    pub fn natural_width(&self) -> i32 {
        self::props::natural_width(self)
    }

    #[qjs(get, rename = "naturalHeight")]
    pub fn natural_height(&self) -> i32 {
        self::props::natural_height(self)
    }

    #[qjs(get, rename = "complete")]
    pub fn complete(&self) -> bool {
        self::props::complete(self)
    }

    // IFrame helpers
    #[qjs(get, rename = "contentWindow")]
    pub fn content_window<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if self::props::tag_name(self) == "IFRAME" {
            // Stub: In the future, this should returns a separate Window/Global object
            return Ok(ctx.globals().into_value()); 
        }
        Ok(Value::new_null(ctx))
    }

    #[qjs(get, rename = "contentDocument")]
    pub fn content_document<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if self::props::tag_name(self) == "IFRAME" {
            // Stub: Returns the document inside the iframe
            return ctx.globals().get("document");
        }
        Ok(Value::new_null(ctx))
    }

    #[qjs(get, rename = "dataset")]
    pub fn dataset<'js>(&self, ctx: Ctx<'js>, this: rquickjs::class::This<Class<'js, Element>>) -> Result<Value<'js>> {
        // We use a Proxy in JS to handle camelCase -> data-kebab-case mapping
        let script = r#"
            (element) => {
                return new Proxy({}, {
                    get(target, prop) {
                        if (typeof prop !== 'string') return undefined;
                        const attrName = 'data-' + prop.replace(/[A-Z]/g, m => '-' + m.toLowerCase());
                        return element.getAttribute(attrName);
                    },
                    set(target, prop, value) {
                        if (typeof prop !== 'string') return false;
                        const attrName = 'data-' + prop.replace(/[A-Z]/g, m => '-' + m.toLowerCase());
                        element.setAttribute(attrName, String(value));
                        return true;
                    },
                    ownKeys(target) {
                        const keys = [];
                        const attrs = element.getAttributeNames();
                        for (const name of attrs) {
                            if (name.startsWith('data-')) {
                                const prop = name.slice(5).replace(/-([a-z])/g, (_, m) => m.toUpperCase());
                                keys.push(prop);
                            }
                        }
                        return keys;
                    },
                    getOwnPropertyDescriptor(target, prop) {
                        return { enumerable: true, configurable: true };
                    }
                });
            }
        "#;
        let factory: rquickjs::Function = ctx.eval(script)?;
        let element_instance = this.0.into_value();
        factory.call((element_instance,))
    }

    #[qjs(rename = "submit")]
    pub fn submit(&self) {
        if self::props::tag_name(self) == "FORM" {
            println!("Form submit triggered for element {}", self.index);
            // In a real engine, this would collect data and navigate.
            // For now, we dispatch a 'submit' event.
            self.dispatch_event_internal("submit".to_string());
        }
    }

    #[qjs(rename = "reset")]
    pub fn reset(&self) {
        if self::props::tag_name(self) == "FORM" {
            println!("Form reset triggered for element {}", self.index);
        }
    }

    #[qjs(rename = "focus")]
    pub fn focus(&self) {
        if let Ok(mut dom) = self.dom.lock() {
            dom.active_element = Some(self.index);
        }
        self.dispatch_event_internal("focus".to_string());
    }

    #[qjs(rename = "blur")]
    pub fn blur(&self) {
        if let Ok(mut dom) = self.dom.lock() {
            if dom.active_element == Some(self.index) {
                dom.active_element = None;
            }
        }
        self.dispatch_event_internal("blur".to_string());
    }
    
    #[qjs(rename = "appendChild")]
    pub fn append_child<'js>(&self, ctx: Ctx<'js>, child: Value<'js>) -> Result<Value<'js>> {
        self::hierarchy::append_child(self, ctx, child)
    }

    #[qjs(rename = "removeChild")]
    pub fn remove_child<'js>(&self, ctx: Ctx<'js>, child: Class<'js, Element>) -> Result<Class<'js, Element>> {
        self::hierarchy::remove_child(self, ctx, child)
    }

    #[qjs(rename = "insertBefore")]
    pub fn insert_before<'js>(&self, ctx: Ctx<'js>, child: Value<'js>, ref_child: Value<'js>) -> Result<Value<'js>> {
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

    #[qjs(get, rename = "childElementCount")]
    pub fn child_element_count(&self) -> usize {
        self::hierarchy::child_element_count(self)
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
