use crate::js::bindings::element::Element;
use crate::js::bindings::element::mark_mutation;
use crate::engine::dom::{AceDOM, AceNodeType, AceNode};
use std::collections::HashMap;

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

pub fn set_text_content(el: &Element, text: String) {
    if let Ok(mut dom) = el.dom.lock() {
        // Clear children
        if let Some(node) = dom.nodes.get_mut(el.index) {
            node.children.clear();
        }
        
        // Add new text node
        let new_node_idx = dom.nodes.len();
        dom.nodes.push(AceNode {
            node_type: AceNodeType::Text(text),
            parent: Some(el.index),
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
        });
        
        if let Some(node) = dom.nodes.get_mut(el.index) {
            node.children.push(new_node_idx);
        }
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

pub fn set_attribute(el: &Element, name: String, value: String) {
    if let Ok(mut dom) = el.dom.lock() {
        if let Some(node) = dom.nodes.get_mut(el.index) {
             if let AceNodeType::Element(element) = &mut node.node_type {
                 element.attributes.insert(name, value);
             }
        }
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
        if let Some(node) = dom.nodes.get_mut(el.index) {
             if let AceNodeType::Element(element) = &mut node.node_type {
                 element.attributes.remove(&name);
             }
        }
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
            AceNodeType::Text(t) => return t.clone(),
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
    // TODO: Parse HTML fragment and append to AceDOM
    // For now: clear children and add a text node saying "HTML Content"
    if let Ok(mut dom) = el.dom.lock() {
        if let Some(node) = dom.nodes.get_mut(el.index) {
            node.children.clear();
        }
        
        // This is a stub because implementing a full HTML fragment parser 
        // that integrates into existing AceDOM arena is complex for this step.
        // Ideally we use kuchiki to parse fragment, then convert to AceDOM nodes, then append.
    }
    mark_mutation(el);
}
