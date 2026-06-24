use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn outer_html(el: &Element) -> String {
    if let Ok(dom) = el.dom.lock() {
        return dom.serialize_subtree_text(el.index);
    }
    "".to_string()
}
