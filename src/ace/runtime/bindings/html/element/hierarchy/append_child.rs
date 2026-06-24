use super::*;
use super::mark_mutation;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use rquickjs::{prelude::Rest, Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};


/// TODO: add docs
pub fn append_child<'js>(el: &Element, ctx: Ctx<'js>, child: Value<'js>) -> Result<Value<'js>> {
    append_child_generic(&el.dom, el.index, &el.mutations, ctx, child)
}
