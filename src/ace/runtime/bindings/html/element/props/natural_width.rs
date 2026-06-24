use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn natural_width(el: &Element) -> i32 {
    // Stub: For now return the same as width. In a real engine we'd fetch from the decoded image.
    width(el)
}
