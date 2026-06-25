use super::*;
use crate::ace::engine::dom::{AceDOM, AceNodeType};
use rquickjs::{prelude::Rest, Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};



pub(crate) use self::Element as ElementType;
pub(crate) fn mark_mutation(el: &ElementType) {
    el.mark_mutation();
}
