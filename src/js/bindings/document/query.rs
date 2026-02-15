use crate::js::bindings::document::Document;
use crate::js::bindings::element::Element;
use crate::engine::dom::{AceDOM, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};

pub fn get_element_by_id<'js>(doc: &Document, ctx: Ctx<'js>, id: String) -> Result<Value<'js>> {
    if let Ok(dom) = doc.dom.lock() {
        for (i, node) in dom.nodes.iter().enumerate() {
            if let AceNodeType::Element(element) = &node.node_type {
                if let Some(el_id) = element.attributes.get("id") {
                    if el_id == &id {
                        let element = Element { 
                            dom: doc.dom.clone(),
                            index: i,
                            mutations: doc.mutations.clone(),
                            stylesheet_dirty: doc.stylesheet_dirty.clone(),
                        };
                        let instance = Class::instance(ctx, element)?;
                        return Ok(instance.into_value());
                    }
                }
            }
        }
    }
    Ok(Value::new_null(ctx))
}

pub fn query_selector<'js>(doc: &Document, ctx: Ctx<'js>, selector: String) -> Result<Value<'js>> {
    if let Ok(dom) = doc.dom.lock() {
        for (i, node) in dom.nodes.iter().enumerate() {
            if matches_node_selector(&node.node_type, &selector) {
                let element = Element { 
                            dom: doc.dom.clone(),
                            index: i,
                            mutations: doc.mutations.clone(),
                            stylesheet_dirty: doc.stylesheet_dirty.clone(),
                        };
                let instance = Class::instance(ctx, element)?;
                return Ok(instance.into_value());
            }
        }
    }
    Ok(Value::new_null(ctx))
}

pub fn query_selector_all<'js>(doc: &Document, ctx: Ctx<'js>, selector: String) -> Result<Value<'js>> {
    let array = rquickjs::Array::new(ctx.clone())?;
    
    if let Ok(dom) = doc.dom.lock() {
        let mut idx = 0;
        for (i, node) in dom.nodes.iter().enumerate() {
            if matches_node_selector(&node.node_type, &selector) {
                let element = Element { 
                            dom: doc.dom.clone(),
                            index: i,
                            mutations: doc.mutations.clone(),
                            stylesheet_dirty: doc.stylesheet_dirty.clone(),
                        };
                let instance = Class::instance(ctx.clone(), element)?;
                array.set(idx, instance)?;
                idx += 1;
            }
        }
    }
    Ok(array.into_value())
}

fn matches_node_selector(node_type: &AceNodeType, selector: &str) -> bool {
    if let AceNodeType::Element(element) = node_type {
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
             if element.tag == selector || selector == "*" { return true; }
        }
    }
    false
}
