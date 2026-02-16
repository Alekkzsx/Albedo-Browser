#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct AbortSignal {
    pub aborted: bool,
}

#[rquickjs::methods]
impl AbortSignal {
    #[qjs(constructor)]
    pub fn new() -> Self { Self { aborted: false } }
}

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct AbortController {
    pub signal: AbortSignal,
}

#[rquickjs::methods]
impl AbortController {
    #[qjs(constructor)]
    pub fn new() -> Self { Self { signal: AbortSignal::new() } }
    pub fn abort(&mut self) { self.signal.aborted = true; }
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
        Self { url, method, headers: Headers::new() }
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
        Self { map: HashMap::new() }
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
        }
    }

    pub fn text<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let (promise, resolve, _) = rquickjs::Promise::new(&ctx)?;
        let _ = resolve.call::<(String,), ()>((self.body.clone(),));
        Ok(promise.into_value())
    }

    pub fn json<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let (promise, resolve, reject) = rquickjs::Promise::new(&ctx)?;
        let json: rquickjs::Object = ctx.globals().get("JSON")?;
        let parse: rquickjs::Function = json.get("parse")?;
        // Specify return type for call explicitly
        match parse.call::<(String,), Value<'js>>((self.body.clone(),)) {
            Ok(val) => { let _ = resolve.call::<(Value<'js>,), ()>((val,)); }
            Err(e) => { let _ = reject.call::<(String,), ()>((e.to_string(),)); }
        }
        Ok(promise.into_value())
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

pub fn register(rt: &JsRuntime) -> rquickjs::Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            
            // Set globals
            global.set("Headers", Class::<Headers>::register(ctx.clone())?)?;
            global.set("Request", Class::<Request>::register(ctx.clone())?)?;
            global.set("AbortController", Class::<AbortController>::register(ctx.clone())?)?;
            global.set("AbortSignal", Class::<AbortSignal>::register(ctx.clone())?)?;
            
            let rt_clone = rt.clone();
            let internal_fetch = rquickjs::Function::new(ctx.clone(), move |url: String, options: rquickjs::Object, resolvers: rquickjs::Array| -> Result<()> {
                let ctx = resolvers.ctx();
                let resolve: Function = resolvers.get(0)?;
                let reject: Function = resolvers.get(1)?;
                
                // Extract options
                let method = options.get::<_, String>("method").unwrap_or_else(|_| "GET".to_string()).to_uppercase();
                let body = options.get::<_, Option<String>>("body").unwrap_or(None);
                let headers_obj = options.get::<_, Option<rquickjs::Object>>("headers").unwrap_or(None);
                
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

                let (id, sender) = {
                    let mut el = rt_clone.event_loop.lock().unwrap();
                    let id = el.register_promise(
                        Persistent::save(ctx, resolve),
                        Persistent::save(ctx, reject)
                    );
                    (id, el.async_sender.clone())
                };

                tokio::spawn(async move {
                    let client = reqwest::Client::new();
                    let mut req_builder = match method.as_str() {
                        "POST" => client.post(&url),
                        "PUT" => client.put(&url),
                        "DELETE" => client.delete(&url),
                        "PATCH" => client.patch(&url),
                        _ => client.get(&url),
                    };

                    for (k, v) in headers_map {
                        req_builder = req_builder.header(k, v);
                    }

                    if let Some(b) = body {
                        req_builder = req_builder.body(b);
                    }

                    let result = req_builder.send().await;
                    let final_result: std::result::Result<(u16, String), String> = match result {
                        Ok(resp) => {
                            let status = resp.status().as_u16();
                            let body = resp.text().await.unwrap_or_default();
                            Ok((status, body))
                        }
                        Err(e) => Err(e.to_string()),
                    };

                    let _ = sender.send(AsyncResult {
                        id,
                        result: final_result,
                    });
                });

                Ok(())
            })?;

            global.set("__internal_fetch", internal_fetch)?;

            ctx.eval::<(), _>(r#"
                globalThis.fetch = function(url, options) {
                    options = options || {};
                    return new Promise((resolve, reject) => {
                        __internal_fetch(url, options, [resolve, reject]);
                    });
                };
            "#)?;
            Ok(())
        })
    })
}
