use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn scroll_top(el: &Element) -> f32 {
    let scroll = el.element_scroll.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, y)) = scroll.get(&el.index) {
        return *y;
    }
    0.0
}
