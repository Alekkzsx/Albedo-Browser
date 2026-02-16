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
