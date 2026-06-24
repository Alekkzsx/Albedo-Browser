use super::*;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{prelude::*, ArrayBuffer, Class, Ctx, Function, Object, Persistent, Result, Value};
// No UnsafeSendVal needed here after synchronous refactor
use std::sync::{Arc, Mutex};



#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct File {
    #[qjs(skip_trace)]
    pub(crate) blob: Blob,
    pub name: String,
    #[qjs(rename = "lastModified")]
    pub last_modified: i64,
}

#[rquickjs::methods]
impl File {
    #[qjs(constructor)]
    pub fn new(
        ctx: Ctx<'_>,
        parts: Option<Value<'_>>,
        name: String,
        options: Option<Object<'_>>,
    ) -> Result<Self> {
        let blob = Blob::new(ctx, parts, options.clone())?;
        let mut last_modified = crate::utils::time::unix_timestamp_millis();
        if let Some(opts) = options {
            last_modified = opts.get("lastModified").unwrap_or(last_modified);
        }
        Ok(Self {
            blob,
            name,
            last_modified,
        })
    }

    #[qjs(get)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[qjs(get, rename = "lastModified")]
    pub fn last_modified(&self) -> i64 {
        self.last_modified
    }

    #[qjs(get)]
    pub fn size(&self) -> usize {
        self.blob.size()
    }

    #[qjs(get, rename = "type")]
    pub fn mime_type(&self) -> String {
        self.blob.mime_type()
    }

    #[qjs(rename = "arrayBuffer")]
    pub fn array_buffer<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.blob.array_buffer(ctx)
    }

    /// TODO: add docs
    pub fn text<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.blob.text(ctx)
    }
}
