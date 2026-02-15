use crate::js::bindings::element::Element;
use crate::engine::dom::{AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};
use crate::js::bindings::element::mark_mutation;

pub fn append_child<'js>(el: &Element, _ctx: Ctx<'js>, child: Class<'js, Element>) -> Result<Class<'js, Element>> {
    let child_borrow = child.borrow();
    let child_idx = child_borrow.index;

    if let Ok(mut dom) = el.dom.lock() {
        // 1. Detach from old parent
        let mut old_parent_idx = None;
        if let Some(child_node) = dom.nodes.get(child_idx) {
            old_parent_idx = child_node.parent;
        }

        if let Some(p_idx) = old_parent_idx {
             if let Some(parent_node) = dom.nodes.get_mut(p_idx) {
                 parent_node.children.retain(|&x| x != child_idx);
             }
        }

        // 2. Attach to new parent
        if let Some(child_node) = dom.nodes.get_mut(child_idx) {
            child_node.parent = Some(el.index);
        }
        
        if let Some(parent_node) = dom.nodes.get_mut(el.index) {
             parent_node.children.push(child_idx);
        }
    }

    mark_mutation(el);
    Ok(child.clone())
}

pub fn remove_child<'js>(el: &Element, _ctx: Ctx<'js>, child: Class<'js, Element>) -> Result<Class<'js, Element>> {
    let child_borrow = child.borrow();
    let child_idx = child_borrow.index;

    if let Ok(mut dom) = el.dom.lock() {
        let mut is_child = false;
        
        if let Some(child_node) = dom.nodes.get(child_idx) {
             if child_node.parent == Some(el.index) {
                 is_child = true;
             }
        }

        if is_child {
            // Remove from parent's children list
             if let Some(parent_node) = dom.nodes.get_mut(el.index) {
                 parent_node.children.retain(|&x| x != child_idx);
             }
             // Clear child's parent ptr
             if let Some(child_node) = dom.nodes.get_mut(child_idx) {
                 child_node.parent = None;
             }
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
