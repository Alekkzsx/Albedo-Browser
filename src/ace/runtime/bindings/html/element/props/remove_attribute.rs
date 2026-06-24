use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn remove_attribute(el: &Element, name: String) {
    if let Ok(mut dom) = el.dom.lock() {
        dom.remove_attribute_notify(el.index, name);
    }
    mark_mutation(el);
}
