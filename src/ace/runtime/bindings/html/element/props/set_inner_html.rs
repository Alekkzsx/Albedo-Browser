use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn set_inner_html(el: &Element, html: String) {
    if let Ok(mut dom) = el.dom.lock() {
        dom.set_inner_html_from_html(el.index, &html);
    }
    mark_mutation(el);
}
