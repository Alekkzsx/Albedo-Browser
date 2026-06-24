use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn set_scroll_top(el: &Element, val: f32) {
    let mut scroll = el.element_scroll.lock().unwrap_or_else(|e| e.into_inner());
    let entry = scroll.entry(el.index).or_insert((0.0, 0.0));
    entry.1 = val;
    // Trigger repaint
    if let Ok(mut sd) = el.stylesheet_dirty.lock() {
        *sd = true;
    }
}
