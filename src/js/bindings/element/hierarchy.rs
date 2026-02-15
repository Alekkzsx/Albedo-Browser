use crate::js::bindings::element::Element;
use crate::engine::dom::{AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};
use crate::js::bindings::element::mark_mutation;

pub fn append_child<'js>(el: &Element, _ctx: Ctx<'js>, child: Class<'js, Element>) -> Result<Class<'js, Element>> {
    let child_borrow = child.borrow();
    let child_idx = child_borrow.index;
    let parent_idx = el.index;

    if let Ok(mut dom) = el.dom.lock() {
        dom.append_child(parent_idx, child_idx);
    }

    mark_mutation(el);
    Ok(child.clone())
}

pub fn remove_child<'js>(el: &Element, _ctx: Ctx<'js>, child: Class<'js, Element>) -> Result<Class<'js, Element>> {
    let child_borrow = child.borrow();
    let child_idx = child_borrow.index;

    if let Ok(mut dom) = el.dom.lock() {
        let is_child = dom.nodes.get(child_idx)
            .map(|n| n.parent == Some(el.index))
            .unwrap_or(false);

        if is_child {
            dom.remove_node_from_parent(child_idx);
            mark_mutation(el);
            return Ok(child.clone());
        }
    }
    
    Err(rquickjs::Error::new_from_js("NotFoundError", "The node to be removed is not a child of this node"))
}

pub fn children<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    let array = rquickjs::Array::new(ctx.clone())?;
    
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
             let mut i = 0;
             for &child_idx in &node.children {
                 if let Some(child_node) = dom.get_node(child_idx) {
                     // Check if Element type
                     if let AceNodeType::Element(_) = child_node.node_type {
                         let element = Element { 
                            dom: el.dom.clone(),
                            index: child_idx,
                            mutations: el.mutations.clone(),
                            stylesheet_dirty: el.stylesheet_dirty.clone(),
                        };
                        let instance = Class::instance(ctx.clone(), element)?;
                        array.set(i, instance)?;
                        i += 1;
                     }
                 }
             }
        }
    }
    Ok(array.into_value())
}

pub fn parent_element<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
             if let Some(parent_idx) = node.parent {
                 // Check if parent is element (usually yes, unless root #document)
                 if let Some(parent_node) = dom.get_node(parent_idx) {
                      if let AceNodeType::Element(_) = parent_node.node_type {
                           let element = Element { 
                                dom: el.dom.clone(),
                                index: parent_idx,
                                mutations: el.mutations.clone(),
                                stylesheet_dirty: el.stylesheet_dirty.clone(),
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

pub fn first_element_child<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
             for &child_idx in &node.children {
                 if let Some(child_node) = dom.get_node(child_idx) {
                     if let AceNodeType::Element(_) = child_node.node_type {
                          let element = Element { 
                                dom: el.dom.clone(),
                                index: child_idx,
                                mutations: el.mutations.clone(),
                                stylesheet_dirty: el.stylesheet_dirty.clone(),
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

pub fn last_element_child<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
             // Iterate reversed
             for &child_idx in node.children.iter().rev() {
                 if let Some(child_node) = dom.get_node(child_idx) {
                     if let AceNodeType::Element(_) = child_node.node_type {
                          let element = Element { 
                                dom: el.dom.clone(),
                                index: child_idx,
                                mutations: el.mutations.clone(),
                                stylesheet_dirty: el.stylesheet_dirty.clone(),
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

pub fn next_element_sibling<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
     if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
             let mut curr = node.next_sibling;
             while let Some(sibling_idx) = curr {
                 if let Some(sibling_node) = dom.get_node(sibling_idx) {
                      if let AceNodeType::Element(_) = sibling_node.node_type {
                           let element = Element { 
                                dom: el.dom.clone(),
                                index: sibling_idx,
                                mutations: el.mutations.clone(),
                                stylesheet_dirty: el.stylesheet_dirty.clone(),
                           };
                           let instance = Class::instance(ctx, element)?;
                           return Ok(instance.into_value());
                      }
                      curr = sibling_node.next_sibling;
                 } else {
                     break;
                 }
             }
        }
     }
    Ok(Value::new_null(ctx))
}

pub fn previous_element_sibling<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
     if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
             let mut curr = node.prev_sibling;
             while let Some(sibling_idx) = curr {
                 if let Some(sibling_node) = dom.get_node(sibling_idx) {
                      if let AceNodeType::Element(_) = sibling_node.node_type {
                           let element = Element { 
                                dom: el.dom.clone(),
                                index: sibling_idx,
                                mutations: el.mutations.clone(),
                                stylesheet_dirty: el.stylesheet_dirty.clone(),
                           };
                           let instance = Class::instance(ctx, element)?;
                           return Ok(instance.into_value());
                      }
                      curr = sibling_node.prev_sibling;
                 } else {
                     break;
                 }
             }
        }
     }
    Ok(Value::new_null(ctx))
}

pub fn insert_before<'js>(el: &Element, ctx: Ctx<'js>, child: Class<'js, Element>, ref_child: Value<'js>) -> Result<Class<'js, Element>> {
    let child_idx = child.borrow().index;
    let parent_idx = el.index;
    
    let ref_idx: Option<usize> = if ref_child.is_null() || ref_child.is_undefined() {
        None
    } else {
        let ref_el = Class::<Element>::from_value(&ref_child)
            .map_err(|_| rquickjs::Error::new_from_js("TypeError", "Argument 2 must be an Element or null"))?;
        let idx = ref_el.borrow().index;
        Some(idx)
    };

    if let Ok(mut dom) = el.dom.lock() {
        dom.insert_before(parent_idx, child_idx, ref_idx);
    }

    mark_mutation(el);
    Ok(child.clone())
}
