use super::*;
use super::mark_mutation;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use rquickjs::{prelude::Rest, Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};


/// TODO: add docs
pub fn insert_adjacent_html<'js>(
    el: &Element,
    _ctx: Ctx<'js>,
    position: String,
    html: String,
) -> Result<()> {
    if let Ok(mut dom) = el.dom.lock() {
        dom.insert_adjacent_html(el.index, &position, &html);
    }
    mark_mutation(el);
    Ok(())
}
