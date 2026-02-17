use rquickjs::{Ctx, Class, Result, Value};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Selection {
    pub is_collapsed: bool,
}

#[rquickjs::methods]
impl Selection {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            is_collapsed: true,
        }
    }

    #[qjs(rename = "getRangeAt")]
    pub fn get_range_at<'js>(&self, ctx: Ctx<'js>, _index: usize) -> Result<Value<'js>> {
        let range = super::range::Range::new();
        let instance = Class::instance(ctx, range)?;
        Ok(instance.into_value())
    }

    #[qjs(rename = "addRange")]
    pub fn add_range(&self, _range: Value<'_>) {
        // Stub
    }

    #[qjs(rename = "removeAllRanges")]
    pub fn remove_all_ranges(&self) {
        // Stub
    }

    #[qjs(get, rename = "anchorNode")]
    pub fn anchor_node<'js>(&self, ctx: Ctx<'js>) -> Value<'js> {
        Value::new_null(ctx)
    }

    #[qjs(get, rename = "anchorOffset")]
    pub fn anchor_offset(&self) -> usize {
        0
    }

    #[qjs(get, rename = "focusNode")]
    pub fn focus_node<'js>(&self, ctx: Ctx<'js>) -> Value<'js> {
        Value::new_null(ctx)
    }

    #[qjs(get, rename = "focusOffset")]
    pub fn focus_offset(&self) -> usize {
        0
    }
}
