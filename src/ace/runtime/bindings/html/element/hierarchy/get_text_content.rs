use super::*;
use super::mark_mutation;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use rquickjs::{prelude::Rest, Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn get_text_content(el: &Element) -> String {
    if let Ok(dom) = el.dom.lock() {
        return dom.serialize_subtree_text(el.index);
    }
    String::new()
}
