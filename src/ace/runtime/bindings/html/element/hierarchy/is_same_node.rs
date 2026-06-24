use super::*;
use super::mark_mutation;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use rquickjs::{prelude::Rest, Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn is_same_node(el: &Element, other: Value) -> bool {
    if let Ok(other_el) = Class::<Element>::from_value(&other) {
        return el.index == other_el.borrow().index;
    }
    false
}
