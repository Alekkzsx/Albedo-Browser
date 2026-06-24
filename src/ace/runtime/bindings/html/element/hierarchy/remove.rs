use super::*;
use super::mark_mutation;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use rquickjs::{prelude::Rest, Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn remove(el: &Element) {
    if let Ok(mut dom) = el.dom.lock() {
        dom.remove_node_from_parent(el.index);
    }
    mark_mutation(el);
}
