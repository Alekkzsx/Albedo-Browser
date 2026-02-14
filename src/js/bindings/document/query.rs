use crate::js::bindings::document::Document;
use crate::js::bindings::element::Element;
use rquickjs::{Class, Ctx, Result, Value};

pub fn get_element_by_id<'js>(doc: &Document, ctx: Ctx<'js>, id: String) -> Result<Value<'js>> {
    if let Some(node) = doc.dom.find_by_id(&id) {
            let element = Element { 
                node,
                mutations: doc.mutations.clone(),
                stylesheet_dirty: doc.stylesheet_dirty.clone(),
            };
            let instance = Class::instance(ctx, element)?;
            Ok(instance.into_value())
    } else {
            Ok(Value::new_null(ctx))
    }
}

pub fn query_selector<'js>(doc: &Document, ctx: Ctx<'js>, selector: String) -> Result<Value<'js>> {
    if let Ok(mut match_iter) = doc.dom.root.select(&selector) {
            if let Some(node_match) = match_iter.next() {
                let element = Element { 
                    node: node_match.as_node().clone(),
                    mutations: doc.mutations.clone(),
                    stylesheet_dirty: doc.stylesheet_dirty.clone(),
                };
                return Class::instance(ctx, element).map(|i| i.into_value());
            }
    }
    Ok(Value::new_null(ctx))
}

pub fn query_selector_all<'js>(doc: &Document, ctx: Ctx<'js>, selector: String) -> Result<Value<'js>> {
    let array = rquickjs::Array::new(ctx.clone())?;
    if let Ok(match_iter) = doc.dom.root.select(&selector) {
        for (i, node_match) in match_iter.enumerate() {
            let element = Element { 
                node: node_match.as_node().clone(),
                mutations: doc.mutations.clone(),
                stylesheet_dirty: doc.stylesheet_dirty.clone(),
            };
            let instance = Class::instance(ctx.clone(), element)?;
            array.set(i, instance)?;
        }
    }
    Ok(array.into_value())
}
