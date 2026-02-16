use rquickjs::{Class, Ctx, Result, Value, Persistent, Object};
use url::Url;
use crate::runtime::bindings::webapi::url_search_params::URLSearchParams;
use std::sync::{Arc, Mutex};

#[rquickjs::class]
#[derive(Clone, rquickjs::class::Trace)]
pub struct URL {
    #[qjs(skip_trace)]
    pub inner: Url,
    pub search_params: Class<'static, URLSearchParams>,
}

#[rquickjs::methods]
impl URL {
    #[qjs(constructor)]
    pub fn new<'js>(ctx: Ctx<'js>, url_str: String, base: Option<String>) -> Result<Self> {
        let url = if let Some(base_str) = base {
            let base_url = Url::parse(&base_str)
                .map_err(|_| rquickjs::Error::new_from_js("Invalid base URL", "TypeError"))?;
            base_url.join(&url_str)
                .map_err(|_| rquickjs::Error::new_from_js("Invalid URL", "TypeError"))?
        } else {
            Url::parse(&url_str)
                .map_err(|_| rquickjs::Error::new_from_js("Invalid URL", "TypeError"))?
        };

        let search = url.query().unwrap_or("");
        let sp = URLSearchParams {
            params: search.split('&')
                .filter(|s| !s.is_empty())
                .map(|pair| {
                    if let Some((k, v)) = pair.split_once('=') {
                        (urlencoding::decode(k).unwrap_or_default().to_string(), 
                         urlencoding::decode(v).unwrap_or_default().to_string())
                    } else {
                        (urlencoding::decode(pair).unwrap_or_default().to_string(), String::new())
                    }
                })
                .collect(),
        };

        let sp_class = Class::instance(ctx.clone(), sp)?;
        // We need 'static for the Class field in struct, so we save it
        // Note: In real rquickjs usage, we'd handle lifetimes more carefully or use Persistent
        let sp_persistent: Class<'static, URLSearchParams> = unsafe { std::mem::transmute(sp_class) };

        Ok(Self {
            inner: url,
            search_params: sp_persistent,
        })
    }

    #[qjs(get)]
    pub fn href(&self) -> String {
        self.inner.to_string()
    }

    #[qjs(set)]
    pub fn set_href<'js>(&mut self, ctx: Ctx<'js>, val: String) -> Result<()> {
        let new_url = Url::parse(&val).map_err(|_| rquickjs::Error::new_from_js("Invalid URL", "TypeError"))?;
        self.inner = new_url;
        
        // Synchronize searchParams
        let search = self.inner.query().unwrap_or("");
        let sp = URLSearchParams {
            params: search.split('&')
                .filter(|s| !s.is_empty())
                .map(|pair| {
                    if let Some((k, v)) = pair.split_once('=') {
                        (urlencoding::decode(k).unwrap_or_default().to_string(), 
                         urlencoding::decode(v).unwrap_or_default().to_string())
                    } else {
                        (urlencoding::decode(pair).unwrap_or_default().to_string(), String::new())
                    }
                })
                .collect(),
        };
        let sp_class = rquickjs::Class::instance(ctx, sp)?;
        let sp_persistent: rquickjs::Class<'static, URLSearchParams> = unsafe { std::mem::transmute(sp_class) };
        self.search_params = sp_persistent;
        Ok(())
    }

    #[qjs(get)]
    pub fn origin(&self) -> String {
        self.inner.origin().unicode_serialization()
    }

    #[qjs(get)]
    pub fn protocol(&self) -> String {
        format!("{}:", self.inner.scheme())
    }

    #[qjs(get)]
    pub fn host(&self) -> String {
        format!("{}{}", self.inner.host_str().unwrap_or(""), self.inner.port().map(|p| format!(":{}", p)).unwrap_or_default())
    }

    #[qjs(get)]
    pub fn hostname(&self) -> String {
        self.inner.host_str().unwrap_or("").to_string()
    }

    #[qjs(get)]
    pub fn port(&self) -> String {
        self.inner.port().map(|p| p.to_string()).unwrap_or_default()
    }

    #[qjs(get)]
    pub fn pathname(&self) -> String {
        self.inner.path().to_string()
    }

    #[qjs(get)]
    pub fn search(&self) -> String {
        let q = self.inner.query().unwrap_or("");
        if q.is_empty() { String::new() } else { format!("?{}", q) }
    }

    #[qjs(get)]
    pub fn hash(&self) -> String {
        let h = self.inner.fragment().unwrap_or("");
        if h.is_empty() { String::new() } else { format!("#{}", h) }
    }

    #[qjs(get, rename = "searchParams")]
    pub fn search_params<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(self.search_params.clone().into_value())
    }

    pub fn to_json(&self) -> String {
        self.inner.to_string()
    }
    
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}

pub fn register(rt: &crate::runtime::core::runtime::JsRuntime) -> Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            let globals = ctx.globals();
            Class::<URL>::register(ctx.clone())?;
            globals.set("URL", Class::<URL>::register(ctx.clone())?)?;
            Ok(())
        })
    })
}
