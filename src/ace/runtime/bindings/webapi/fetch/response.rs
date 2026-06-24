use super::*;
use crate::ace::runtime::core::event_loop::AsyncResult;
use crate::ace::runtime::core::service_worker::{
    CacheMode, InterceptResult, RedirectMode, RequestContext,
};
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;



#[derive(rquickjs::class::Trace, Clone)]
#[rquickjs::class(rename = "Response")]
pub struct Response {
    pub status: u16,
    #[qjs(skip_trace)]
    pub body: String,
    pub headers: Headers,
}

#[rquickjs::methods]
impl Response {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            status: 200,
            body: String::new(),
            headers: Headers::new(),
        }
    }

    /// TODO: add docs
    pub fn text<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let p = rquickjs::Promise::new(&ctx)?;
        let _ = p.1.call::<(String,), ()>((self.body.clone(),));
        Ok(p.0.into_value())
    }

    /// TODO: add docs
    pub fn json<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let p = rquickjs::Promise::new(&ctx)?;
        let json: rquickjs::Object = ctx.globals().get("JSON")?;
        let parse: rquickjs::Function = json.get("parse")?;
        match parse.call::<(String,), Value<'js>>((self.body.clone(),)) {
            Ok(val) => {
                let _ = p.1.call::<(Value<'js>,), ()>((val,));
            }
            Err(e) => {
                let _ = p.2.call::<(String,), ()>((e.to_string(),));
            }
        }
        Ok(p.0.into_value())
    }

    #[qjs(get)]
    pub fn ok(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    #[qjs(get, rename = "statusText")]
    pub fn status_text(&self) -> String {
        match self.status {
            200 => "OK".to_string(),
            201 => "Created".to_string(),
            400 => "Bad Request".to_string(),
            401 => "Unauthorized".to_string(),
            403 => "Forbidden".to_string(),
            404 => "Not Found".to_string(),
            500 => "Internal Server Error".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    #[qjs(get)]
    pub fn headers<'js>(&self, ctx: Ctx<'js>) -> Result<Class<'js, Headers>> {
        Class::instance(ctx, self.headers.clone())
    }
}
