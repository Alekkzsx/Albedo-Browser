use crate::runtime::bindings::webapi::url_search_params::URLSearchParams;
use crate::runtime::core::runtime::JsRuntime;
use rquickjs::{prelude::*, Class, Ctx, Object, Persistent, Result, Value};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct URL {
    #[qjs(skip_trace)]
    pub(crate) url: crate::ace::url::Url,
    #[qjs(skip_trace)]
    pub(crate) search_params: Persistent<Object<'static>>,
}

#[rquickjs::methods]
impl URL {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>, input: String, base: Option<String>) -> Result<Self> {
        let url = if let Some(base_str) = base {
            let base_url = crate::ace::url::parse(&base_str, None)
                .map_err(|_| rquickjs::Error::new_from_js("URL", "Invalid base URL"))?;
            base_url
                .join(&input)
                .map_err(|_| rquickjs::Error::new_from_js("URL", "Invalid URL parsing"))?
        } else {
            crate::ace::url::parse(&input, None)
                .map_err(|_| rquickjs::Error::new_from_js("URL", "Invalid URL"))?
        };

        let query = url.query().unwrap_or("").to_string();
        let query_js = query.into_js(&ctx)?;
        let search_params = URLSearchParams::new(ctx.clone(), Some(query_js))?;
        let search_params_class = Class::instance(ctx.clone(), search_params)?;
        let search_params_obj =
            <rquickjs::Value<'_> as Clone>::clone(&search_params_class).into_object();

        Ok(URL {
            url,
            search_params: Persistent::save(&ctx, search_params_obj.unwrap()),
        })
    }

    #[qjs(get)]
    pub fn href(&self) -> String {
        self.url.to_string()
    }

    #[qjs(set, rename = "href")]
    pub fn href_setter(&mut self, val: String) -> Result<()> {
        self.url = crate::ace::url::parse(&val, None)
            .map_err(|_| rquickjs::Error::new_from_js("URL", "Invalid URL"))?;
        Ok(())
    }

    #[qjs(get)]
    pub fn origin(&self) -> String {
        self.url.origin()
    }

    #[qjs(get)]
    pub fn protocol(&self) -> String {
        format!("{}:", self.url.scheme())
    }

    #[qjs(get)]
    pub fn host(&self) -> String {
        format!(
            "{}{}",
            self.url.host_str().unwrap_or_default(),
            self.url
                .port()
                .map(|p| format!(":{}", p))
                .unwrap_or_default()
        )
    }

    #[qjs(get)]
    pub fn hostname(&self) -> String {
        self.url.host_str().unwrap_or_default().to_string()
    }

    #[qjs(get)]
    pub fn port(&self) -> String {
        self.url.port().map(|p| p.to_string()).unwrap_or_default()
    }

    #[qjs(get)]
    pub fn pathname(&self) -> String {
        self.url.path().to_string()
    }

    #[qjs(get)]
    pub fn search(&self) -> String {
        self.url
            .query()
            .map(|q| format!("?{}", q))
            .unwrap_or_default()
    }

    #[qjs(get, rename = "searchParams")]
    pub fn search_params<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.search_params
            .clone()
            .restore(&ctx)
            .map(|obj: Object<'js>| obj.into_value())
    }

    #[qjs(get)]
    pub fn hash(&self) -> String {
        self.url
            .fragment()
            .map(|f| format!("#{}", f))
            .unwrap_or_default()
    }
}

pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            ctx.globals().set("URL", Class::<URL>::register(&ctx)?)?;
            Ok(())
        })
    })
}
