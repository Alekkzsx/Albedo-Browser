use crate::js::bindings::element::Element;
use rquickjs::{Class, Ctx, Result, Value};
use kuchiki::traits::*;
use crate::js::bindings::element::mark_mutation;

pub fn append_child<'js>(el: &Element, _ctx: Ctx<'js>, child: Class<'js, Element>) -> Result<Class<'js, Element>> {
    let child_borrow = child.borrow();
    el.node.append(child_borrow.node.clone());
    mark_mutation(el);
    Ok(child.clone())
}

pub fn remove_child<'js>(el: &Element, _ctx: Ctx<'js>, child: Class<'js, Element>) -> Result<Class<'js, Element>> {
    let child_borrow = child.borrow();
    
    if let Some(parent) = child_borrow.node.parent() {
        if parent == el.node {
            child_borrow.node.detach();
            mark_mutation(el);
            return Ok(child.clone());
        }
    }
    
    Err(rquickjs::Error::new_from_js("NotFoundError", "The node to be removed is not a child of this node"))
}

pub fn children<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    let array = rquickjs::Array::new(ctx.clone())?;
    let mut i = 0;
    for child in el.node.children() {
        if child.as_element().is_some() {
            let element = Element { 
                node: child,
                mutations: el.mutations.clone(),
                stylesheet_dirty: el.stylesheet_dirty.clone(),
            };
            let instance = Class::instance(ctx.clone(), element)?;
            array.set(i, instance)?;
            i += 1;
        }
    }
    Ok(array.into_value())
}

pub fn parent_element<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    if let Some(parent) = el.node.parent() {
        if parent.as_element().is_some() {
            let element = Element { 
                node: parent,
                mutations: el.mutations.clone(),
                stylesheet_dirty: el.stylesheet_dirty.clone(),
            };
            let instance = Class::instance(ctx, element)?;
            return Ok(instance.into_value());
        }
    }
    Ok(Value::new_null(ctx))
}

pub fn first_element_child<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    for child in el.node.children() {
        if child.as_element().is_some() {
            let element = Element { 
                node: child,
                mutations: el.mutations.clone(),
                stylesheet_dirty: el.stylesheet_dirty.clone(),
            };
            let instance = Class::instance(ctx, element)?;
            return Ok(instance.into_value());
        }
    }
    Ok(Value::new_null(ctx))
}

pub fn last_element_child<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    let mut last_el = None;
    for child in el.node.children() {
        if child.as_element().is_some() {
            last_el = Some(child);
        }
    }

    if let Some(node) = last_el {
        let element = Element { 
            node,
            mutations: el.mutations.clone(),
            stylesheet_dirty: el.stylesheet_dirty.clone(),
        };
        let instance = Class::instance(ctx, element)?;
        return Ok(instance.into_value());
    }
    Ok(Value::new_null(ctx))
}

pub fn next_element_sibling<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    let mut curr = el.node.next_sibling();
    while let Some(node) = curr {
        if node.as_element().is_some() {
            let element = Element { 
                node,
                mutations: el.mutations.clone(),
                stylesheet_dirty: el.stylesheet_dirty.clone(),
            };
            let instance = Class::instance(ctx, element)?;
            return Ok(instance.into_value());
        }
        curr = node.next_sibling();
    }
    Ok(Value::new_null(ctx))
}

pub fn previous_element_sibling<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    let mut curr = el.node.previous_sibling();
    while let Some(node) = curr {
        if node.as_element().is_some() {
            let element = Element { 
                node,
                mutations: el.mutations.clone(),
                stylesheet_dirty: el.stylesheet_dirty.clone(),
            };
            let instance = Class::instance(ctx, element)?;
            return Ok(instance.into_value());
        }
        curr = node.previous_sibling();
    }
    Ok(Value::new_null(ctx))
}
