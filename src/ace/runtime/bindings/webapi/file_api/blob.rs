use super::*;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{prelude::*, ArrayBuffer, Class, Ctx, Function, Object, Persistent, Result, Value};
// No UnsafeSendVal needed here after synchronous refactor
use std::sync::{Arc, Mutex};


#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Blob {
    #[qjs(skip_trace)]
    pub(crate) data: Vec<u8>,
    pub(crate) mime_type: String,
}

#[rquickjs::methods]
impl Blob {
    #[qjs(constructor)]
    pub fn new(
        _ctx: Ctx<'_>,
        parts: Option<Value<'_>>,
        options: Option<Object<'_>>,
    ) -> Result<Self> {
        let mut data = Vec::new();
        if let Some(parts_val) = parts {
            if let Some(arr) = parts_val.as_array() {
                for i in 0..arr.len() {
                    let part: Value = arr.get(i)?;
                    if let Some(s) = part.as_string() {
                        data.extend_from_slice(s.to_string()?.as_bytes());
                    } else if let Some(blob_obj) = part.as_object() {
                        if let Some(blob) = Class::<Blob>::from_object(&blob_obj) {
                            data.extend_from_slice(&blob.borrow().data);
                        }
                    } else if let Some(ab) = part.as_object().and_then(|obj| obj.as_array_buffer())
                    {
                        data.extend_from_slice(ab.as_ref());
                    }
                }
            }
        }

        let mut mime_type = String::new();
        if let Some(opts) = options {
            mime_type = opts.get("type").unwrap_or_default();
        }

        Ok(Self { data, mime_type })
    }

    #[qjs(get)]
    pub fn size(&self) -> usize {
        self.data.len()
    }

    #[qjs(get, rename = "type")]
    pub fn mime_type(&self) -> String {
        self.mime_type.clone()
    }

    /// TODO: add docs
    pub fn slice(
        &self,
        start: Option<isize>,
        end: Option<isize>,
        content_type: Option<String>,
    ) -> Self {
        let len = self.data.len() as isize;
        let start = start.unwrap_or(0);
        let end = end.unwrap_or(len);

        let s = if start < 0 {
            (len + start).max(0)
        } else {
            start.min(len)
        } as usize;
        let e = if end < 0 {
            (len + end).max(0)
        } else {
            end.min(len)
        } as usize;

        let data = if s < e {
            self.data[s..e].to_vec()
        } else {
            Vec::new()
        };

        Self {
            data,
            mime_type: content_type.unwrap_or_default(),
        }
    }

    #[qjs(rename = "arrayBuffer")]
    pub fn array_buffer<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let ab = ArrayBuffer::new(ctx.clone(), self.data.clone())?;
        let _ = resolve.call::<(ArrayBuffer<'js>,), ()>((ab,));
        Ok(promise.into_value())
    }

    /// TODO: add docs
    pub fn text<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let text = String::from_utf8_lossy(&self.data).to_string();
        let _ = resolve.call::<(String,), ()>((text,));
        Ok(promise.into_value())
    }
}
