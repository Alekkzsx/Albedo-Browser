use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};

pub fn tag_name(el: &Element) -> String {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            if let AceNodeType::Element(element) = &node.node_type {
                return element.tag.to_uppercase();
            }
        }
    }
    "".to_string()
}

pub fn id(el: &Element) -> String {
    get_attribute(el, "id".to_string()).unwrap_or_default()
}

pub fn set_id(el: &Element, val: String) {
    set_attribute(el, "id".to_string(), val);
}

pub fn class_name(el: &Element) -> String {
    get_attribute(el, "class".to_string()).unwrap_or_default()
}

pub fn set_class_name(el: &Element, val: String) {
    set_attribute(el, "class".to_string(), val);
}

pub fn name(el: &Element) -> String {
    get_attribute(el, "name".to_string()).unwrap_or_default()
}

pub fn set_name(el: &Element, val: String) {
    set_attribute(el, "name".to_string(), val);
}

pub fn title_prop(el: &Element) -> String {
    get_attribute(el, "title".to_string()).unwrap_or_default()
}

pub fn set_title_prop(el: &Element, val: String) {
    set_attribute(el, "title".to_string(), val);
}

pub fn src(el: &Element) -> String {
    get_attribute(el, "src".to_string()).unwrap_or_default()
}

pub fn set_src(el: &Element, val: String) {
    set_attribute(el, "src".to_string(), val);
}

pub fn href(el: &Element) -> String {
    get_attribute(el, "href".to_string()).unwrap_or_default()
}

pub fn set_href(el: &Element, val: String) {
    set_attribute(el, "href".to_string(), val);
}

pub fn value(el: &Element) -> String {
    get_attribute(el, "value".to_string()).unwrap_or_default()
}

pub fn set_value(el: &Element, val: String) {
    set_attribute(el, "value".to_string(), val);
}

pub fn checked(el: &Element) -> bool {
    get_attribute(el, "checked".to_string()).is_some()
}

pub fn set_checked(el: &Element, val: bool) {
    if val {
        set_attribute(el, "checked".to_string(), "checked".to_string());
    } else {
        remove_attribute(el, "checked".to_string());
    }
}

pub fn width(el: &Element) -> i32 {
    get_attribute(el, "width".to_string())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

pub fn set_width(el: &Element, val: i32) {
    set_attribute(el, "width".to_string(), val.to_string());
}

pub fn height(el: &Element) -> i32 {
    get_attribute(el, "height".to_string())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

pub fn set_height(el: &Element, val: i32) {
    set_attribute(el, "height".to_string(), val.to_string());
}

pub fn natural_width(el: &Element) -> i32 {
    // Stub: For now return the same as width. In a real engine we'd fetch from the decoded image.
    width(el)
}

pub fn natural_height(el: &Element) -> i32 {
    // Stub: For now return the same as height.
    height(el)
}

pub fn complete(_el: &Element) -> bool {
    // Stub: Assume loaded for now.
    true
}

pub fn text_content(el: &Element) -> String {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            return collect_text(&dom, node);
        }
    }
    "".to_string()
}

fn collect_text(dom: &AceDOM, node: &AceNode) -> String {
    let mut s = String::new();
    if let AceNodeType::Text(text) = &node.node_type {
        s.push_str(text);
    }
    for &child_idx in &node.children {
        if let Some(child) = dom.get_node(child_idx) {
            s.push_str(&collect_text(dom, child));
        }
    }
    s
}

pub fn inner_text(el: &Element) -> String {
    // textContent is a good approximation for innerText in this engine
    text_content(el)
}

pub fn set_text_content(el: &Element, text: String) {
    if let Ok(mut dom) = el.dom.lock() {
        dom.set_text_content_notify(el.index, text);
    }
    mark_mutation(el);
}

pub fn get_attribute(el: &Element, name: String) -> Option<String> {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            if let AceNodeType::Element(element) = &node.node_type {
                return element.attributes.get(&name).cloned();
            }
        }
    }
    None
}

pub fn get_attribute_names(el: &Element) -> Vec<String> {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            if let AceNodeType::Element(element) = &node.node_type {
                return element.attributes.keys().cloned().collect();
            }
        }
    }
    Vec::new()
}

pub fn set_attribute(el: &Element, name: String, value: String) {
    if let Ok(mut dom) = el.dom.lock() {
        dom.set_attribute_notify(el.index, name, value);
    }
    mark_mutation(el);
}

pub fn has_attribute(el: &Element, name: String) -> bool {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            if let AceNodeType::Element(element) = &node.node_type {
                return element.attributes.contains_key(&name);
            }
        }
    }
    false
}

pub fn remove_attribute(el: &Element, name: String) {
    if let Ok(mut dom) = el.dom.lock() {
        dom.remove_attribute_notify(el.index, name);
    }
    mark_mutation(el);
}

pub fn inner_html(el: &Element) -> String {
    // Basic serialization stub
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            // Serialize children
            let mut s = String::new();
            for &child_idx in &node.children {
                s.push_str(&serialize_node(&dom, child_idx));
            }
            return s;
        }
    }
    "".to_string()
}

fn serialize_node(dom: &AceDOM, node_idx: usize) -> String {
    if let Some(node) = dom.get_node(node_idx) {
        match &node.node_type {
            AceNodeType::Text(t) => return t.to_string(),
            AceNodeType::Element(el) => {
                let mut s = format!("<{}", el.tag);
                for (k, v) in &el.attributes {
                    s.push_str(&format!(" {}=\"{}\"", k, v));
                }
                s.push_str(">");
                for &child_idx in &node.children {
                    s.push_str(&serialize_node(dom, child_idx));
                }
                s.push_str(&format!("</{}>", el.tag));
                return s;
            }
            _ => return "".to_string(),
        }
    }
    "".to_string()
}

pub fn set_inner_html(el: &Element, html: String) {
    if let Ok(mut dom) = el.dom.lock() {
        dom.set_inner_html_from_html(el.index, &html);
    }
    mark_mutation(el);
}

pub fn outer_html(el: &Element) -> String {
    if let Ok(dom) = el.dom.lock() {
        return dom.serialize_subtree_text(el.index);
    }
    "".to_string()
}

pub fn set_outer_html(el: &Element, html: String) {
    // outerHTML setter is more complex: it replaces the element itself.
    // Spec: "On setting, the element will be replaced by a fragment which is created by parsing the given string."
    if let Ok(mut dom) = el.dom.lock() {
        let parent_idx = dom.get_node(el.index).and_then(|n| n.parent);
        if let Some(p_idx) = parent_idx {
            let ref_idx = dom.get_node(el.index).and_then(|n| n.next_sibling);

            // Remove old node
            dom.remove_node_from_parent(el.index);

            // Insert new nodes from fragment
            for child_idx in dom.import_html_fragment(&html, Some(p_idx)) {
                dom.insert_before(p_idx, child_idx, ref_idx);
            }
        }
    }
    mark_mutation(el);
}

pub fn offset_parent<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    // Naive: just return parent if it is an element
    super::hierarchy::parent_element(el, ctx)
}

pub fn offset_top(el: &Element) -> f32 {
    let geometry = el.element_geometry.lock().unwrap();
    if let Some(geom) = geometry.get(&el.index) {
        return geom.y;
    }
    0.0
}

pub fn offset_left(el: &Element) -> f32 {
    let geometry = el.element_geometry.lock().unwrap();
    if let Some(geom) = geometry.get(&el.index) {
        return geom.x;
    }
    0.0
}

pub fn offset_width(el: &Element) -> f32 {
    let geometry = el.element_geometry.lock().unwrap();
    if let Some(geom) = geometry.get(&el.index) {
        return geom.width;
    }
    0.0
}

pub fn offset_height(el: &Element) -> f32 {
    let geometry = el.element_geometry.lock().unwrap();
    if let Some(geom) = geometry.get(&el.index) {
        return geom.height;
    }
    0.0
}

pub fn client_top(el: &Element) -> f32 {
    let geometry = el.element_geometry.lock().unwrap();
    if let Some(geom) = geometry.get(&el.index) {
        return geom.border_top;
    }
    0.0
}

pub fn client_left(el: &Element) -> f32 {
    let geometry = el.element_geometry.lock().unwrap();
    if let Some(geom) = geometry.get(&el.index) {
        return geom.border_left;
    }
    0.0
}

pub fn client_width(el: &Element) -> f32 {
    let geometry = el.element_geometry.lock().unwrap();
    if let Some(geom) = geometry.get(&el.index) {
        return geom.client_width();
    }
    0.0
}

pub fn client_height(el: &Element) -> f32 {
    let geometry = el.element_geometry.lock().unwrap();
    if let Some(geom) = geometry.get(&el.index) {
        return geom.client_height();
    }
    0.0
}

pub fn scroll_top(el: &Element) -> f32 {
    let scroll = el.element_scroll.lock().unwrap();
    if let Some((_, y)) = scroll.get(&el.index) {
        return *y;
    }
    0.0
}

pub fn set_scroll_top(el: &Element, val: f32) {
    let mut scroll = el.element_scroll.lock().unwrap();
    let entry = scroll.entry(el.index).or_insert((0.0, 0.0));
    entry.1 = val;
    // Trigger repaint
    if let Ok(mut sd) = el.stylesheet_dirty.lock() {
        *sd = true;
    }
}

pub fn scroll_left(el: &Element) -> f32 {
    let scroll = el.element_scroll.lock().unwrap();
    if let Some((x, _)) = scroll.get(&el.index) {
        return *x;
    }
    0.0
}

pub fn set_scroll_left(el: &Element, val: f32) {
    let mut scroll = el.element_scroll.lock().unwrap();
    let entry = scroll.entry(el.index).or_insert((0.0, 0.0));
    entry.0 = val;
    if let Ok(mut sd) = el.stylesheet_dirty.lock() {
        *sd = true;
    }
}

pub fn scroll_width(el: &Element) -> f32 {
    let geometry = el.element_geometry.lock().unwrap();
    if let Some(geom) = geometry.get(&el.index) {
        return geom.scroll_width();
    }
    0.0
}

pub fn scroll_height(el: &Element) -> f32 {
    let geometry = el.element_geometry.lock().unwrap();
    if let Some(geom) = geometry.get(&el.index) {
        return geom.scroll_height();
    }
    0.0
}

pub fn attach_shadow<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    if let Ok(mut dom) = el.dom.lock() {
        let shadow_idx = dom.attach_shadow(el.index);

        let element = Element {
            dom: el.dom.clone(),
            index: shadow_idx,
            mutations: el.mutations.clone(),
            stylesheet_dirty: el.stylesheet_dirty.clone(),
            primitives: el.primitives.clone(),
            canvas_contexts: el.canvas_contexts.clone(),
            pending_scroll: el.pending_scroll.clone(),
            element_geometry: el.element_geometry.clone(),
            element_scroll: el.element_scroll.clone(),
        };
        let instance = Class::instance(ctx, element)?;
        return Ok(instance.into_value());
    }
    // Erro ao travar mutex
    Err(rquickjs::Error::Unknown)
}
