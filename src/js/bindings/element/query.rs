use crate::js::bindings::element::Element;
use rquickjs::{Class, Ctx, Result, Value};
use kuchiki::traits::*;

pub fn query_selector<'js>(el: &Element, ctx: Ctx<'js>, selector: String) -> Result<Value<'js>> {
    if let Ok(mut match_iter) = el.node.select(&selector) {
            if let Some(node_match) = match_iter.next() {
                let element = Element { 
                    node: node_match.as_node().clone(),
                    mutations: el.mutations.clone(),
                    stylesheet_dirty: el.stylesheet_dirty.clone(),
                };
                return Class::instance(ctx, element).map(|i| i.into_value());
            }
    }
    Ok(Value::new_null(ctx))
}

pub fn query_selector_all<'js>(el: &Element, ctx: Ctx<'js>, selector: String) -> Result<Value<'js>> {
    let array = rquickjs::Array::new(ctx.clone())?;
    if let Ok(match_iter) = el.node.select(&selector) {
        for (i, node_match) in match_iter.enumerate() {
            let element = Element { 
                node: node_match.as_node().clone(),
                mutations: el.mutations.clone(),
                stylesheet_dirty: el.stylesheet_dirty.clone(),
            };
            let instance = Class::instance(ctx.clone(), element)?;
            array.set(i, instance)?;
        }
    }
    Ok(array.into_value())
}
