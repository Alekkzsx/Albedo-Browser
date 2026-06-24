use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn set_height(el: &Element, val: i32) {
    set_attribute(el, "height".to_string(), val.to_string());
}
