use rquickjs::{Ctx, Class, Value, Result};
use super::{Element, mark_mutation};
use crate::engine::dom::{AceDOM, AceNodeType, AceNode};
use kuchiki::traits::*;
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
            shadow_root: None,
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
        // Need a remove_attribute_notify in AceDOM?
        // For simplicity, let's just implement it here or call a notify
        let mut old_value = None;
        if let Some(node) = dom.nodes.get_mut(el.index) {
             if let AceNodeType::Element(element) = &mut node.node_type {
                 old_value = element.attributes.remove(&name);
             }
        }
        
        dom.notify_mutation(el.index, crate::engine::dom::MutationRecord {
            type_: "attributes".to_string(),
            target: el.index,
            added_nodes: vec![],
            removed_nodes: vec![],
            previous_sibling: None,
            next_sibling: None,
            attribute_name: Some(name),
            old_value,
        });
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
    if let Ok(mut dom) = el.dom.lock() {
        // Parse HTML como um documento completo (mais simples que fragmento no kuchiki)
        let kuchiki_root = kuchiki::parse_html().from_utf8().one(html.as_bytes());
        
        // Encontrar o body do fragmento analisado
        if let Ok(body_match) = kuchiki_root.select_first("body") {
            let body_node = body_match.as_node().clone();
            dom.set_inner_html_from_kuchiki(el.index, body_node.children());
        } else {
            // Se não houver body (ex: texto puro ou fragmento sem tags estruturais), 
            // kuchiki_root costuma ter o conteúdo no html ou diretamente.
            // Vamos tentar pegar os filhos da raiz se o body falhar.
            dom.set_inner_html_from_kuchiki(el.index, kuchiki_root.children());
        }
    }
    mark_mutation(el);
}
pub fn attach_shadow<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    if let Ok(mut dom) = el.dom.lock() {
        let shadow_idx = dom.attach_shadow(el.index);
        
        let element = Element { 
            dom: el.dom.clone(),
            index: shadow_idx,
            mutations: el.mutations.clone(),
            stylesheet_dirty: el.stylesheet_dirty.clone(),
        };
        let instance = Class::instance(ctx, element)?;
        return Ok(instance.into_value());
    }
    // Erro ao travar mutex
    Err(rquickjs::Error::Unknown)
}
