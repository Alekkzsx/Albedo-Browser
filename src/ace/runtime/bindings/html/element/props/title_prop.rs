use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn title_prop(el: &Element) -> String {
    get_attribute(el, "title".to_string()).unwrap_or_default()
}
