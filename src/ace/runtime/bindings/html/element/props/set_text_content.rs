use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn set_text_content(el: &Element, text: String) {
    if let Ok(mut dom) = el.dom.lock() {
        dom.set_text_content_notify(el.index, text);
    }
    mark_mutation(el);
}
