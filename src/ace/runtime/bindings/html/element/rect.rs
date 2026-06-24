#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct DOMRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[rquickjs::methods]
impl DOMRect {
    #[qjs(constructor)]
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            top: y,
            left: x,
            bottom: y + height,
            right: x + width,
        }
    }
}

use super::Element;
use rquickjs::{Class, Ctx, Result, Value};

/// TODO: add docs
pub fn get_bounding_client_rect<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    let primitives = el.primitives.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(prim) = primitives.iter().find(|p| p.node_idx == el.index) {
        let rect = DOMRect::new(prim.x, prim.y, prim.width, prim.height);
        let instance = Class::instance(ctx, rect)?;
        return Ok(instance.into_value());
    }
    // Default zero rect
    let rect = DOMRect::new(0.0, 0.0, 0.0, 0.0);
    let instance = Class::instance(ctx, rect)?;
    Ok(instance.into_value())
}
