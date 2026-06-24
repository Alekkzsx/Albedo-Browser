use super::*;
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{prelude::*, ArrayBuffer, Class, Ctx, Function, Object, Persistent, Result, Value};
// No UnsafeSendVal needed here after synchronous refactor
use std::sync::{Arc, Mutex};



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
        let rt = Class::<JsRuntime>::from_object(rt_val.as_object().expect("Albedo Engine: internal invariant violated"))
            .expect("Albedo Engine: internal invariant violated")
            .borrow()
            .clone();

        let blob_data = blob.data.clone();

        // Perform read synchronously (data is already in memory)
        let content = String::from_utf8_lossy(&blob_data).to_string();
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                let js_val = value.clone().into_js(&ctx).expect("Albedo Engine: internal invariant violated");
                *self.result.lock().unwrap_or_else(|e| e.into_inner()) = Some(Persistent::save(&ctx, js_val));
                if let Some(ref cb) = *self.onload.lock().unwrap_or_else(|e| e.into_inner()) {
                    if let Ok(func) = cb.clone().restore(&ctx) {
                        let event = rquickjs::Object::new(ctx.clone()).expect("Albedo Engine: internal invariant violated");
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
        let rt = Class::<JsRuntime>::from_object(rt_val.as_object().expect("Albedo Engine: internal invariant violated"))
            .expect("Albedo Engine: internal invariant violated")
            .borrow()
            .clone();

        let blob_data = blob.data.clone();
        let mime_type = blob.mime_type.clone();

        // Perform conversion synchronously (data in memory)
        let base64_str = crate::utils::base64::encode(&blob_data);
        let result_str = format!("data:{};base64,{}", mime_type, base64_str);
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                let js_val = result_str.clone().into_js(&ctx).expect("Albedo Engine: internal invariant violated");
                *self.result.lock().unwrap_or_else(|e| e.into_inner()) = Some(Persistent::save(&ctx, js_val));
                if let Some(ref cb) = *self.onload.lock().unwrap_or_else(|e| e.into_inner()) {
                    if let Ok(func) = cb.clone().restore(&ctx) {
                        let event = rquickjs::Object::new(ctx.clone()).expect("Albedo Engine: internal invariant violated");
                        let _ = func.call::<(Object,), ()>((event,));
                    }
                }
            })
        });
        Ok(())
    }

    #[qjs(get)]
    pub fn result<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(ref res) = *self.result.lock().unwrap_or_else(|e| e.into_inner()) {
            res.clone().restore(&ctx)
        } else {
            Ok(Value::new_null(ctx))
        }
    }

    #[qjs(get, rename = "onload")]
    pub fn onload_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(ref cb) = *self.onload.lock().unwrap_or_else(|e| e.into_inner()) {
            cb.clone().restore(&ctx).map(|f| f.into_value())
        } else {
            Ok(Value::new_null(ctx))
        }
    }

    #[qjs(set, rename = "onload")]
    pub fn onload_setter<'js>(&self, ctx: Ctx<'js>, func: Function<'js>) -> Result<()> {
        *self.onload.lock().unwrap_or_else(|e| e.into_inner()) = Some(Persistent::save(&ctx, func));
        Ok(())
    }

    #[qjs(get, rename = "onerror")]
    pub fn onerror_get<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(ref cb) = *self.onerror.lock().unwrap_or_else(|e| e.into_inner()) {
            cb.clone().restore(&ctx).map(|f| f.into_value())
        } else {
            Ok(Value::new_null(ctx))
        }
    }

    #[qjs(set, rename = "onerror")]
    pub fn onerror_setter<'js>(&self, ctx: Ctx<'js>, func: Function<'js>) -> Result<()> {
        *self.onerror.lock().unwrap_or_else(|e| e.into_inner()) = Some(Persistent::save(&ctx, func));
        Ok(())
    }
}
