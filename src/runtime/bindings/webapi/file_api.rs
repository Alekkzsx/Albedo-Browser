use crate::runtime::core::runtime::JsRuntime;
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

    pub fn text<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let text = String::from_utf8_lossy(&self.data).to_string();
        let _ = resolve.call::<(String,), ()>((text,));
        Ok(promise.into_value())
    }
}

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
        let mut last_modified = chrono::Utc::now().timestamp_millis();
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

    pub fn text<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.blob.text(ctx)
    }
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct FileReader {
    #[qjs(skip_trace)]
    onload: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
    #[qjs(skip_trace)]
    onerror: Arc<Mutex<Option<Persistent<Function<'static>>>>>,
    #[qjs(skip_trace)]
    result: Arc<Mutex<Option<Persistent<Value<'static>>>>>,
}

#[rquickjs::methods]
impl FileReader {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            onload: Arc::new(Mutex::new(None)),
            onerror: Arc::new(Mutex::new(None)),
            result: Arc::new(Mutex::new(None)),
        }
    }

    #[qjs(rename = "readAsText")]
    pub fn read_as_text(&self, ctx: Ctx<'_>, blob_val: Value<'_>) -> Result<()> {
        let blob_obj = blob_val
            .as_object()
            .ok_or_else(|| rquickjs::Error::new_from_js("Blob", "Expected Object"))?;
        let blob = Class::<Blob>::from_object(blob_obj)
            .ok_or_else(|| rquickjs::Error::new_from_js("Blob", "Invalid Blob object"))?
            .borrow()
            .clone();
        let rt_val = ctx.globals().get::<_, Value>("__albedo_rt__")?;
        let rt = Class::<JsRuntime>::from_object(rt_val.as_object().unwrap())
            .unwrap()
            .borrow()
            .clone();

        let blob_data = blob.data.clone();

        // Perform read synchronously (data is already in memory)
        let val = String::from_utf8_lossy(&blob_data).to_string();
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                let js_val = val.clone().into_js(&ctx).unwrap();
                *self.result.lock().unwrap() = Some(Persistent::save(&ctx, js_val));
                if let Some(ref cb) = *self.onload.lock().unwrap() {
                    if let Ok(func) = cb.clone().restore(&ctx) {
                        let event = rquickjs::Object::new(ctx.clone()).unwrap();
                        let _ = func.call::<(Object,), ()>((event,));
                    }
                }
            })
        });
        Ok(())
    }

    #[qjs(rename = "readAsDataURL")]
    pub fn read_as_data_url(&self, ctx: Ctx<'_>, blob_val: Value<'_>) -> Result<()> {
        let blob_obj = blob_val
            .as_object()
            .ok_or_else(|| rquickjs::Error::new_from_js("Blob", "Expected Object"))?;
        let blob = Class::<Blob>::from_object(blob_obj)
            .ok_or_else(|| rquickjs::Error::new_from_js("Blob", "Invalid Blob object"))?
            .borrow()
            .clone();
        let rt_val = ctx.globals().get::<_, Value>("__albedo_rt__")?;
        let rt = Class::<JsRuntime>::from_object(rt_val.as_object().unwrap())
            .unwrap()
            .borrow()
            .clone();

        let blob_data = blob.data.clone();
        let mime_type = blob.mime_type.clone();

        // Perform conversion synchronously (data in memory)
        let base64_str = crate::ace::util::base64::encode(&blob_data);
        let result_str = format!("data:{};base64,{}", mime_type, base64_str);
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                let js_val = result_str.clone().into_js(&ctx).unwrap();
                *self.result.lock().unwrap() = Some(Persistent::save(&ctx, js_val));
                if let Some(ref cb) = *self.onload.lock().unwrap() {
                    if let Ok(func) = cb.clone().restore(&ctx) {
                        let event = rquickjs::Object::new(ctx.clone()).unwrap();
                        let _ = func.call::<(Object,), ()>((event,));
                    }
                }
            })
        });
        Ok(())
    }

    #[qjs(get)]
    pub fn result<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(ref res) = *self.result.lock().unwrap() {
            res.clone().restore(&ctx)
        } else {
            Ok(Value::new_null(ctx))
        }
    }

    #[qjs(get, rename = "onload")]
    pub fn onload_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(ref cb) = *self.onload.lock().unwrap() {
            cb.clone().restore(&ctx).map(|f| f.into_value())
        } else {
            Ok(Value::new_null(ctx))
        }
    }

    #[qjs(set, rename = "onload")]
    pub fn onload_setter<'js>(&self, ctx: Ctx<'js>, func: Function<'js>) -> Result<()> {
        *self.onload.lock().unwrap() = Some(Persistent::save(&ctx, func));
        Ok(())
    }

    #[qjs(get, rename = "onerror")]
    pub fn onerror_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(ref cb) = *self.onerror.lock().unwrap() {
            cb.clone().restore(&ctx).map(|f| f.into_value())
        } else {
            Ok(Value::new_null(ctx))
        }
    }

    #[qjs(set, rename = "onerror")]
    pub fn onerror_setter<'js>(&self, ctx: Ctx<'js>, func: Function<'js>) -> Result<()> {
        *self.onerror.lock().unwrap() = Some(Persistent::save(&ctx, func));
        Ok(())
    }
}

pub fn register(rt: &JsRuntime) -> rquickjs::Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            let globals = ctx.globals();
            globals.set("Blob", Class::<Blob>::register(&ctx)?)?;
            globals.set("File", Class::<File>::register(&ctx)?)?;
            globals.set("FileReader", Class::<FileReader>::register(&ctx)?)?;
            Ok(())
        })
    })
}
