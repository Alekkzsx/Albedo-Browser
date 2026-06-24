use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn height(el: &Element) -> i32 {
    get_attribute(el, "height".to_string())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}
