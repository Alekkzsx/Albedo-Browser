use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn set_checked(el: &Element, val: bool) {
    if val {
        set_attribute(el, "checked".to_string(), "checked".to_string());
    } else {
        remove_attribute(el, "checked".to_string());
    }
}
