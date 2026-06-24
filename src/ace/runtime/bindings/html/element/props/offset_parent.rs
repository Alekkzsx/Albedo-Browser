use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn offset_parent<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    // Naive: just return parent if it is an element
    super::hierarchy::parent_element(el, ctx)
}
