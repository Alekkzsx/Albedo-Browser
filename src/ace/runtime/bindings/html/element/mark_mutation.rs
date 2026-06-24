use super::*;
use crate::ace::engine::dom::{AceDOM, AceNodeType};
use rquickjs::{prelude::Rest, Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};



pub mod attributes;
pub mod canvas;
pub mod canvas_context;
pub mod dataset;
pub mod events;
pub mod hierarchy;
pub mod props;
pub mod query;
pub mod rect;
pub mod shadow;
pub mod style;

pub(crate) use self::Element as ElementType;
pub(crate) fn mark_mutation(el: &ElementType) {
    el.mark_mutation();
}
