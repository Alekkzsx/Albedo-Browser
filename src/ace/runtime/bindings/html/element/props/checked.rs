use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn checked(el: &Element) -> bool {
    get_attribute(el, "checked".to_string()).is_some()
}
