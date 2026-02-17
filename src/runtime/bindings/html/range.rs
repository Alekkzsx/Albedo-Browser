use rquickjs::{Ctx, Class, Result, Value};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Range {
    pub start_offset: usize,
    pub end_offset: usize,
    pub collapsed: bool,
}

#[rquickjs::methods]
impl Range {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            start_offset: 0,
            end_offset: 0,
            collapsed: true,
        }
    }

    #[qjs(rename = "setStart")]
    pub fn set_start(&mut self, _node: Value<'_>, offset: usize) {
        self.start_offset = offset;
    }

    #[qjs(rename = "setEnd")]
    pub fn set_end(&mut self, _node: Value<'_>, offset: usize) {
        self.end_offset = offset;
    }

    #[qjs(rename = "collapse")]
    pub fn collapse(&mut self, _to_start: bool) {
        self.collapsed = true;
    }

    #[qjs(rename = "selectNode")]
    pub fn select_node(&mut self, _node: Value<'_>) {
        // Stub
    }

    #[qjs(rename = "cloneRange")]
    pub fn clone_range(&self) -> Self {
        self.clone()
    }

    #[qjs(rename = "detach")]
    pub fn detach(&self) {
        // Legacy
    }
}
