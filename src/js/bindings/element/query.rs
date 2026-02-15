use crate::js::bindings::element::Element;
use crate::engine::dom::{AceDOM, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};

pub fn query_selector<'js>(el: &Element, ctx: Ctx<'js>, selector: String) -> Result<Value<'js>> {
    if let Ok(dom) = el.dom.lock() {
        if let Some(found_idx) = find_element(&dom, el.index, &selector, true) {
             let element = Element { 
                dom: el.dom.clone(),
                index: found_idx,
                mutations: el.mutations.clone(),
                stylesheet_dirty: el.stylesheet_dirty.clone(),
            };
            let instance = Class::instance(ctx, element)?;
            return Ok(instance.into_value());
        }
    }
    Ok(Value::new_null(ctx))
}

pub fn query_selector_all<'js>(el: &Element, ctx: Ctx<'js>, selector: String) -> Result<Value<'js>> {
    let array = rquickjs::Array::new(ctx.clone())?;
    if let Ok(dom) = el.dom.lock() {
        let found_indices = find_elements(&dom, el.index, &selector);
        for (i, &idx) in found_indices.iter().enumerate() {
            let element = Element { 
                dom: el.dom.clone(),
                index: idx,
                mutations: el.mutations.clone(),
                stylesheet_dirty: el.stylesheet_dirty.clone(),
            };
            let instance = Class::instance(ctx.clone(), element)?;
            array.set(i, instance)?;
        }
    }
    Ok(array.into_value())
}

// Helper: recursive finder
fn find_element(dom: &AceDOM, root_idx: usize, selector: &str, is_first: bool) -> Option<usize> {
    if let Some(node) = dom.get_node(root_idx) {
        // Check current node (but usually querySelector on element doesn't match itself, it matches descendants)
        // MDN: "The Document method querySelector() returns the first Element within the document that matches the specified selector, or group of selectors. If no matches are found, null is returned."
        // Element.querySelector: "The Element method querySelector() returns the first element that is a descendant of the element on which it is invoked that matches the specified group of selectors."
        
        // So we skip the root_idx itself check if we are calling this recursively.
        // But for recursion simplified: we iterate children.
        
        for &child_idx in &node.children {
            if matches_selector(dom, child_idx, selector) {
                return Some(child_idx);
            }
            if let Some(found) = find_element(dom, child_idx, selector, false) {
                return Some(found);
            }
        }
    }
    None
}

fn find_elements(dom: &AceDOM, root_idx: usize, selector: &str) -> Vec<usize> {
    let mut results = Vec::new();
    if let Some(node) = dom.get_node(root_idx) {
        for &child_idx in &node.children {
            if matches_selector(dom, child_idx, selector) {
                results.push(child_idx);
            }
            results.extend(find_elements(dom, child_idx, selector));
        }
    }
    results
}

fn matches_selector(dom: &AceDOM, node_idx: usize, selector: &str) -> bool {
    if let Some(node) = dom.get_node(node_idx) {
        if let AceNodeType::Element(element) = &node.node_type {
            if selector.starts_with('#') {
                let id = &selector[1..];
                if let Some(el_id) = element.attributes.get("id") {
                    if el_id == id { return true; }
                }
            } else if selector.starts_with('.') {
                let class_name = &selector[1..];
                if let Some(el_class) = element.attributes.get("class") {
                    for c in el_class.split_whitespace() {
                        if c == class_name { return true; }
                    }
                }
            } else {
                 if element.tag == selector || (selector == "*" ) { return true; }
            }
        }
    }
    false
}
