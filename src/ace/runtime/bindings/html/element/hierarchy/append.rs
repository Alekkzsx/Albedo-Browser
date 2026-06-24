use super::*;
use super::mark_mutation;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use rquickjs::{prelude::Rest, Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn append<'js>(el: &Element, ctx: Ctx<'js>, nodes: Rest<Value<'js>>) -> Result<()> {
    for val in nodes.0 {
        append_child(el, ctx.clone(), val)?;
    }
    Ok(())
}
