use super::Element;
use crate::engine::dom::{AceNodeType};
use rquickjs::{Class, Ctx, Result, Value, prelude::Rest};
use super::mark_mutation;
use std::sync::{Arc, Mutex};

pub fn append_child<'js>(el: &Element, ctx: Ctx<'js>, child: Value<'js>) -> Result<Value<'js>> {
    append_child_generic(&el.dom, el.index, &el.mutations, ctx, child)
}

pub fn append_child_generic<'js>(
    dom_mutex: &Arc<Mutex<crate::engine::dom::AceDOM>>,
    parent_idx: usize,
    mutations: &Arc<Mutex<bool>>,
    _ctx: Ctx<'js>,
    child: Value<'js>
) -> Result<Value<'js>> {
    if let Ok(child_el) = Class::<Element>::from_value(&child) {
        let child_borrow = child_el.borrow();
        let child_idx = child_borrow.index;

        if let Ok(mut dom) = dom_mutex.lock() {
            dom.append_child(parent_idx, child_idx);
        }

        if let Ok(mut m) = mutations.lock() {
            *m = true;
        }
        return Ok(child);
    } else if let Ok(fragment) = Class::<crate::runtime::bindings::html::document::fragment::DocumentFragment>::from_value(&child) {
        let fragment_borrow = fragment.borrow();
        let fragment_idx = fragment_borrow.index;

        if let Ok(mut dom) = dom_mutex.lock() {
            let children_to_move = if let Some(frag_node) = dom.get_node(fragment_idx) {
                frag_node.children.clone()
            } else {
                Vec::new()
            };

            for child_idx in children_to_move {
                dom.append_child(parent_idx, child_idx);
            }
        }

        if let Ok(mut m) = mutations.lock() {
            *m = true;
        }
        return Ok(child);
    }

    Err(rquickjs::Error::new_from_js("TypeError", "Argument 1 must be an Element or DocumentFragment"))
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

pub fn parent_node<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
             if let Some(parent_idx) = node.parent {
                let element = Element { 
                    dom: el.dom.clone(),
                    index: parent_idx,
                    mutations: el.mutations.clone(),
                    stylesheet_dirty: el.stylesheet_dirty.clone(),
                    primitives: el.primitives.clone(),
                };
                let instance = Class::instance(ctx, element)?;
                return Ok(instance.into_value());
             }
        }
    }
    Ok(Value::new_null(ctx))
}

pub fn first_child<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
             if let Some(&child_idx) = node.children.first() {
                let element = Element { 
                    dom: el.dom.clone(),
                    index: child_idx,
                    mutations: el.mutations.clone(),
                    stylesheet_dirty: el.stylesheet_dirty.clone(),
                    primitives: el.primitives.clone(),
                };
                let instance = Class::instance(ctx, element)?;
                return Ok(instance.into_value());
             }
        }
    }
    Ok(Value::new_null(ctx))
}

pub fn last_child<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
             if let Some(&child_idx) = node.children.last() {
                let element = Element { 
                    dom: el.dom.clone(),
                    index: child_idx,
                    mutations: el.mutations.clone(),
                    stylesheet_dirty: el.stylesheet_dirty.clone(),
                    primitives: el.primitives.clone(),
                };
                let instance = Class::instance(ctx, element)?;
                return Ok(instance.into_value());
             }
        }
    }
    Ok(Value::new_null(ctx))
}

pub fn next_sibling<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
             if let Some(sibling_idx) = node.next_sibling {
                let element = Element { 
                    dom: el.dom.clone(),
                    index: sibling_idx,
                    mutations: el.mutations.clone(),
                    stylesheet_dirty: el.stylesheet_dirty.clone(),
                    primitives: el.primitives.clone(),
                };
                let instance = Class::instance(ctx, element)?;
                return Ok(instance.into_value());
             }
        }
    }
    Ok(Value::new_null(ctx))
}

pub fn previous_sibling<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
             if let Some(sibling_idx) = node.prev_sibling {
                let element = Element { 
                    dom: el.dom.clone(),
                    index: sibling_idx,
                    mutations: el.mutations.clone(),
                    stylesheet_dirty: el.stylesheet_dirty.clone(),
                    primitives: el.primitives.clone(),
                };
                let instance = Class::instance(ctx, element)?;
                return Ok(instance.into_value());
             }
        }
    }
    Ok(Value::new_null(ctx))
}

pub fn append<'js>(el: &Element, ctx: Ctx<'js>, nodes: Rest<Value<'js>>) -> Result<()> {
    for val in nodes.0 {
        append_child(el, ctx.clone(), val)?;
    }
    Ok(())
}

pub fn prepend<'js>(el: &Element, ctx: Ctx<'js>, nodes: Rest<Value<'js>>) -> Result<()> {
    let mut ref_idx = None;
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            ref_idx = node.children.first().cloned();
        }
    }

    let ref_val = if let Some(idx) = ref_idx {
        let element = Element { 
            dom: el.dom.clone(),
            index: idx,
            mutations: el.mutations.clone(),
            stylesheet_dirty: el.stylesheet_dirty.clone(),
            primitives: el.primitives.clone(),
        };
        Class::instance(ctx.clone(), element)?.into_value()
    } else {
        Value::new_null(ctx.clone())
    };

    for val in nodes.0 {
        insert_before(el, ctx.clone(), val, ref_val.clone())?;
    }
    Ok(())
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
                            primitives: el.primitives.clone(),
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
                                primitives: el.primitives.clone(),
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
                                primitives: el.primitives.clone(),
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
                                primitives: el.primitives.clone(),
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
                                primitives: el.primitives.clone(),
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
                                primitives: el.primitives.clone(),
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

pub fn child_element_count(el: &Element) -> usize {
    if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            return node.children.iter().filter(|&&idx| {
                if let Some(child) = dom.get_node(idx) {
                    matches!(child.node_type, AceNodeType::Element(_))
                } else {
                    false
                }
            }).count();
        }
    }
    0
}

pub fn insert_before<'js>(el: &Element, ctx: Ctx<'js>, child: Value<'js>, ref_child: Value<'js>) -> Result<Value<'js>> {
    let parent_idx = el.index;
    
    let ref_idx: Option<usize> = if ref_child.is_null() || ref_child.is_undefined() {
        None
    } else {
        let ref_el = Class::<Element>::from_value(&ref_child)
            .map_err(|_| rquickjs::Error::new_from_js("TypeError", "Argument 2 must be an Element or null"))?;
        let idx = ref_el.borrow().index;
        Some(idx)
    };

    if let Ok(child_el) = Class::<Element>::from_value(&child) {
        let child_idx = child_el.borrow().index;
        if let Ok(mut dom) = el.dom.lock() {
            dom.insert_before(parent_idx, child_idx, ref_idx);
        }
        mark_mutation(el);
        return Ok(child);
    } else if let Ok(fragment) = Class::<crate::runtime::bindings::html::document::fragment::DocumentFragment>::from_value(&child) {
        let fragment_borrow = fragment.borrow();
        let fragment_idx = fragment_borrow.index;

        if let Ok(mut dom) = el.dom.lock() {
            // Move all children from fragment to parent before ref_idx
            let children_to_move = if let Some(frag_node) = dom.get_node(fragment_idx) {
                frag_node.children.clone()
            } else {
                Vec::new()
            };

            for child_idx in children_to_move {
                dom.insert_before(parent_idx, child_idx, ref_idx);
            }
        }

        mark_mutation(el);
        return Ok(child);
    }

    Err(rquickjs::Error::new_from_js("TypeError", "Argument 1 must be an Element or DocumentFragment"))
}

pub fn remove(el: &Element) {
    if let Ok(mut dom) = el.dom.lock() {
        dom.remove_node_from_parent(el.index);
    }
    mark_mutation(el);
}

pub fn contains(el: &Element, other: Value) -> bool {
    let other_idx = if let Ok(other_el) = Class::<Element>::from_value(&other) {
        other_el.borrow().index
    } else {
        return false;
    };

    if el.index == other_idx {
        return true;
    }

    if let Ok(dom) = el.dom.lock() {
        let mut curr = other_idx;
        while let Some(node) = dom.get_node(curr) {
            if let Some(parent) = node.parent {
                if parent == el.index {
                    return true;
                }
                curr = parent;
            } else {
                break;
            }
        }
    }
    false
}

pub fn is_same_node(el: &Element, other: Value) -> bool {
    if let Ok(other_el) = Class::<Element>::from_value(&other) {
        return el.index == other_el.borrow().index;
    }
    false
}

pub fn get_text_content(el: &Element) -> String {
    if let Ok(dom) = el.dom.lock() {
        return dom.serialize_subtree_text(el.index);
    }
    String::new()
}

pub fn set_text_content(el: &Element, text: String) {
    if let Ok(mut dom) = el.dom.lock() {
        dom.set_text_content_notify(el.index, text);
    }
    mark_mutation(el);
}
pub fn insert_adjacent_html<'js>(el: &Element, _ctx: Ctx<'js>, position: String, html: String) -> Result<()> {
    if let Ok(mut dom) = el.dom.lock() {
        dom.insert_adjacent_html(el.index, &position, &html);
    }
    mark_mutation(el);
    Ok(())
}
