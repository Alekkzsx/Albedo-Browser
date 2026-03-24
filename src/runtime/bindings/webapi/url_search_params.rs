use rquickjs::{Array, Ctx, Result, Value};

#[rquickjs::class]
#[derive(Clone, rquickjs::class::Trace)]
pub struct URLSearchParams {
    #[qjs(skip_trace)]
    pub params: Vec<(String, String)>,
}

#[rquickjs::methods]
impl URLSearchParams {
    #[qjs(constructor)]
    pub fn new<'js>(_ctx: Ctx<'js>, init: Option<Value<'js>>) -> Result<Self> {
        let mut params = Vec::new();
        if let Some(val) = init {
            if let Some(s) = val.as_string() {
                let query = s.to_string()?;
                let query = query.trim_start_matches('?');
                for pair in query.split('&') {
                    if let Some((k, v)) = pair.split_once('=') {
                        params.push((k.to_string(), v.to_string()));
                    } else if !pair.is_empty() {
                        params.push((pair.to_string(), String::new()));
                    }
                }
            }
        }
        Ok(Self { params })
    }

    pub fn get(&self, name: String) -> Option<String> {
        self.params
            .iter()
            .find(|(k, _)| k == &name)
            .map(|(_, v)| v.clone())
    }

    pub fn set(&mut self, name: String, value: String) {
        if let Some(pos) = self.params.iter().position(|(k, _)| k == &name) {
            self.params[pos].1 = value;
            // Remove subsequent params with same name
            let mut i = pos + 1;
            while i < self.params.len() {
                if self.params[i].0 == name {
                    self.params.remove(i);
                } else {
                    i += 1;
                }
            }
        } else {
            self.params.push((name, value));
        }
    }

    pub fn append(&mut self, name: String, value: String) {
        self.params.push((name, value));
    }

    pub fn delete(&mut self, name: String) {
        self.params.retain(|(k, _)| k != &name);
    }

    pub fn has(&self, name: String) -> bool {
        self.params.iter().any(|(k, _)| k == &name)
    }

    #[qjs(rename = "getAll")]
    pub fn get_all(&self, name: String) -> Vec<String> {
        self.params
            .iter()
            .filter(|(k, _)| k == &name)
            .map(|(_, v)| v.clone())
            .collect()
    }

    pub fn sort(&mut self) {
        self.params.sort_by(|a, b| a.0.cmp(&b.0));
    }

    pub fn entries<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let arr = Array::new(ctx.clone())?;
        for (i, (k, v)) in self.params.iter().enumerate() {
            let pair = Array::new(ctx.clone())?;
            pair.set(0, k.clone())?;
            pair.set(1, v.clone())?;
            arr.set(i, pair)?;
        }
        // In a real browser this returns an iterator, but returning an array
        // that's iterable is a common and functional shortcut for simple engines.
        // We could implement a real iterator class if needed by the user.
        Ok(arr.into_value())
    }

    pub fn keys<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let arr = Array::new(ctx.clone())?;
        for (i, (k, _)) in self.params.iter().enumerate() {
            arr.set(i, k.clone())?;
        }
        Ok(arr.into_value())
    }

    pub fn values<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let arr = Array::new(ctx.clone())?;
        for (i, (_, v)) in self.params.iter().enumerate() {
            arr.set(i, v.clone())?;
        }
        Ok(arr.into_value())
    }

    pub fn to_string(&self) -> String {
        self.params
            .iter()
            .map(|(k, v)| {
                let encoded_k = crate::ace::url::percent_encoding::encode(k, crate::ace::url::percent_encoding::EncodeSet::Query);
                let encoded_v = crate::ace::url::percent_encoding::encode(v, crate::ace::url::percent_encoding::EncodeSet::Query);
                format!("{}={}", encoded_k, encoded_v)
            })
            .collect::<Vec<_>>()
            .join("&")
    }
}

// Ensure the class is iterable in JS
// rquickjs usually allows defining [Symbol.iterator]

pub fn register(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    globals.set(
        "URLSearchParams",
        rquickjs::Class::<URLSearchParams>::register(&ctx.clone())?,
    )?;

    // Base64 (using engine::base64 or similar if available, but for now fixed base64 crate usage)
    use base64::Engine;
    let b64_engine = base64::engine::general_purpose::STANDARD;

    let engine_clone = b64_engine.clone();
    globals.set(
        "btoa",
        rquickjs::Function::new(ctx.clone(), move |s: String| -> String {
            engine_clone.encode(s)
        }),
    )?;

    let engine_clone = b64_engine.clone();
    globals.set(
        "atob",
        rquickjs::Function::new(ctx.clone(), move |s: String| -> Result<String> {
            engine_clone
                .decode(s)
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .map_err(|_| rquickjs::Error::new_from_js("Invalid base64", "Error"))
        }),
    )?;

    Ok(())
}
