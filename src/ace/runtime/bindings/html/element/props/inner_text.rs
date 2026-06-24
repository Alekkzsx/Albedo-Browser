use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn inner_text(el: &Element) -> String {
    // textContent is a good approximation for innerText in this engine
    text_content(el)
}
