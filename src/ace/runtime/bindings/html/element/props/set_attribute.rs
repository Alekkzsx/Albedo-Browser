use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn set_attribute(el: &Element, name: String, value: String) {
    if let Ok(mut dom) = el.dom.lock() {
        dom.set_attribute_notify(el.index, name, value);
    }
    mark_mutation(el);
}
