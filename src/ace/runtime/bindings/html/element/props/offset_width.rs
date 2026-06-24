use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn offset_width(el: &Element) -> f32 {
    let geometry = el.element_geometry.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(geom) = geometry.get(&el.index) {
        return geom.width;
    }
    0.0
}
