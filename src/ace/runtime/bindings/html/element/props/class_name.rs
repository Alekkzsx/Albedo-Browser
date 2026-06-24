use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn class_name(el: &Element) -> String {
    get_attribute(el, "class".to_string()).unwrap_or_default()
}
