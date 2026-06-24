use crate::ace::runtime::core::event_loop::AsyncResult;
use crate::ace::runtime::core::service_worker::{
    CacheMode, InterceptResult, RedirectMode, RequestContext,
};
use rquickjs::{Class, Ctx, Function, Object, Persistent, Result, Value};
use std::collections::HashMap;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct AbortSignal {
    pub aborted: bool,
}

#[rquickjs::methods]
impl AbortSignal {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self { aborted: false }
    }
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct AbortController {
    pub signal: AbortSignal,
}

#[rquickjs::methods]
impl AbortController {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            signal: AbortSignal::new(),
        }
    }
    pub fn abort(&mut self) {
        self.signal.aborted = true;
    }
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Request {
    pub url: String,
    pub method: String,
    pub headers: Headers,
}

#[rquickjs::methods]
impl Request {
    #[qjs(constructor)]
    pub fn new(url: String, options: Option<Object<'_>>) -> Self {
        let mut method = "GET".to_string();
        if let Some(opts) = options {
            method = opts.get("method").unwrap_or("GET".to_string());
        }
        Self {
            url,
            method,
            headers: Headers::new(),
        }
    }
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Headers {
    #[qjs(skip_trace)]
    pub map: HashMap<String, String>,
}

#[rquickjs::methods]
impl Headers {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn get(&self, name: String) -> Option<String> {
        self.map.get(&name.to_lowercase()).cloned()
    }

    pub fn set(&mut self, name: String, value: String) {
        self.map.insert(name.to_lowercase(), value);
    }
}

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

    pub fn text<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let p = rquickjs::Promise::new(&ctx)?;
        let _ = p.1.call::<(String,), ()>((self.body.clone(),));
        Ok(p.0.into_value())
    }

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

use crate::shared::security::Origin;
use crate::ace::runtime::core::runtime::JsRuntime;

fn try_service_worker_intercept(
    sw_manager: &std::sync::Arc<std::sync::Mutex<crate::ace::runtime::core::service_worker::ServiceWorkerManager>>,
    origin_str: &str,
    url: &str,
    method: &str,
    headers: &std::collections::HashMap<String, String>,
) -> Option<crate::ace::runtime::core::service_worker::ResponseContext> {
    if let Ok(Some(reg)) = sw_manager.find_for_url(origin_str, url) {
        if let Ok(Some(active)) = reg.get_active() {
            let req_ctx = RequestContext {
                method: method.to_string(),
                url: url.to_string(),
                headers: headers.clone(),
                body: None,
                mode: "cors".to_string(),
                credentials: "omit".to_string(),
                cache_mode: CacheMode::Default,
                redirect: RedirectMode::Follow,
            };

            if let Ok(InterceptResult::Handled(sw_resp)) = sw_manager.dispatch_fetch_event(&active, req_ctx) {
                return Some(sw_resp);
            }
        }
    }
    None
}

fn validate_cors_response(
    rm: &Option<crate::network::resources::ResourceManager>,
    org: &Option<Origin>,
    url: &str,
    resp_headers: &std::collections::HashMap<String, String>,
) -> bool {
    if let Some(ref rm) = rm {
        if let Some(ref org) = org {
            return rm.access_control.lock().unwrap().validate_cors(org, url, resp_headers);
        }
    }
    false
}

fn extract_headers_from_options(
    options: &rquickjs::Object,
) -> (String, Option<String>, std::collections::HashMap<String, String>) {
    let method = options
        .get::<_, String>("method")
        .unwrap_or_else(|_| "GET".to_string())
        .to_uppercase();
    let body = options.get::<_, Option<String>>("body").unwrap_or(None);
    let headers_obj = options
        .get::<_, Option<rquickjs::Object>>("headers")
        .unwrap_or(None);

    let mut headers_map = std::collections::HashMap::new();
    if let Some(h_obj) = headers_obj {
        for key in h_obj.keys::<String>() {
            if let Ok(k) = key {
                if let Ok(v) = h_obj.get::<_, String>(k.clone()) {
                    headers_map.insert(k, v);
                }
            }
        }
    }
    (method, body, headers_map)
}

async fn execute_fetch(
    url: String,
    method: String,
    body: Option<String>,
    headers_map: std::collections::HashMap<String, String>,
    id: usize,
    sender: tokio::sync::mpsc::UnboundedSender<AsyncResult>,
    origin_arc: std::sync::Arc<std::sync::Mutex<Option<Origin>>>,
    resource_manager: std::sync::Arc<std::sync::Mutex<Option<crate::network::resources::ResourceManager>>>,
    sw_manager: std::sync::Arc<std::sync::Mutex<crate::ace::runtime::core::service_worker::ServiceWorkerManager>>,
) {
    let origin_str = {
        let lock = origin_arc.lock().unwrap();
        lock.as_ref()
            .map(|o| o.to_string())
            .unwrap_or_else(|| "null".to_string())
    };

    if let Some(sw_resp) = try_service_worker_intercept(
        &sw_manager, &origin_str, &url, &method, &headers_map,
    ) {
        let _ = sender.send(AsyncResult {
            id,
            result: Ok((
                sw_resp.status,
                String::from_utf8_lossy(&sw_resp.body).to_string(),
            )),
        });
        return;
    }

    let (rm_opt, org_opt) = {
        let rm_lock = resource_manager.lock().unwrap();
        let org_lock = origin_arc.lock().unwrap();
        ((*rm_lock).clone(), (*org_lock).clone())
    };

    let client = if let Some(ref rm) = rm_opt {
        rm.client.clone()
    } else {
        reqwest::Client::new()
    };

    let target_origin = Origin::from_url(&url);
    let is_cross_origin = match (&org_opt, &target_origin) {
        (Some(o), Some(t)) => !o.is_same_origin(t),
        _ => true,
    };

    let mut req_builder = match method.as_str() {
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        "PATCH" => client.patch(&url),
        _ => client.get(&url),
    };

    if let Some(ref rm) = rm_opt {
        let cookies = rm.cookie_jar.lock().unwrap().get_cookies_for_url(&url);
        if !cookies.is_empty() {
            req_builder = req_builder.header("Cookie", cookies);
        }
    }

    for (k, v) in headers_map {
        req_builder = req_builder.header(k, v);
    }

    if let Some(b) = body {
        req_builder = req_builder.body(b);
    }

    let response_result = req_builder.send().await;

    let final_result: std::result::Result<(u16, String), String> =
        match response_result {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let mut resp_headers = std::collections::HashMap::new();
                for (k, v) in resp.headers().iter() {
                    resp_headers.insert(
                        k.to_string(),
                        v.to_str().unwrap_or("").to_string(),
                    );
                }

                if is_cross_origin {
                    let allowed = validate_cors_response(&rm_opt, &org_opt, &url, &resp_headers);

                    if !allowed {
                        let _ = sender.send(AsyncResult {
                            id,
                            result: Err(
                                "CORS Error: Origin not allowed".to_string()
                            ),
                        });
                        return;
                    }
                }

                if let Some(ref rm) = rm_opt {
                    if let Some(cookie_header) =
                        resp.headers().get("set-cookie")
                    {
                        if let Ok(c_str) = cookie_header.to_str() {
                            rm.cookie_jar
                                .lock()
                                .unwrap()
                                .set_cookie(&url, c_str);
                        }
                    }
                }

                let body = resp.text().await.unwrap_or_default();
                Ok((status, body))
            }
            Err(e) => Err(e.to_string()),
        };

    let _ = sender.send(AsyncResult {
        id,
        result: final_result,
    });
}

pub fn register(rt: &JsRuntime) -> rquickjs::Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            let global = ctx.globals();

            global.set("Headers", Class::<Headers>::register(&ctx)?)?;
            global.set("Request", Class::<Request>::register(&ctx)?)?;
            global.set("AbortController", Class::<AbortController>::register(&ctx)?)?;
            global.set("AbortSignal", Class::<AbortSignal>::register(&ctx)?)?;

            let rt_clone = rt.clone();
            let internal_fetch = rquickjs::Function::new(
                ctx.clone(),
                move |url: String,
                      options: rquickjs::Object,
                      resolvers: rquickjs::Array|
                      -> Result<()> {
                    let ctx = resolvers.ctx();
                    let resolve: Function = resolvers.get(0)?;
                    let reject: Function = resolvers.get(1)?;

                    let (method, body, headers_map) = extract_headers_from_options(&options);

                    let (id, sender) = {
                        let mut el = rt_clone.event_loop.lock().unwrap();
                        let id = el.register_promise(
                            Persistent::save(&ctx, resolve),
                            Persistent::save(&ctx, reject),
                        );
                        (id, el.async_sender.clone())
                    };

                    let origin_arc = rt_clone.origin.clone();
                    let resource_manager = rt_clone.resource_manager.clone();
                    let sw_manager = rt_clone.sw_manager.clone();

                    tokio::spawn(execute_fetch(
                        url, method, body, headers_map, id, sender,
                        origin_arc, resource_manager, sw_manager,
                    ));

                    Ok(())
                },
            )?;

            global.set("__internal_fetch", internal_fetch)?;

            ctx.eval::<(), _>(
                r#"
                globalThis.fetch = function(url, options) {
                    options = options || {};
                    return new Promise((resolve, reject) => {
                        __internal_fetch(url, options, [resolve, reject]);
                    });
                };
            "#,
            )?;
            Ok(())
        })
    })
}
