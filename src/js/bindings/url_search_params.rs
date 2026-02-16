use rquickjs::{Ctx, Result, Value, Object, Array};
use std::collections::HashMap;

#[rquickjs::class]
#[derive(Clone, rquickjs::class::Trace)]
pub struct URLSearchParams {
    #[qjs(skip_trace)]
    pub params: Vec<(String, String)>,
}

#[rquickjs::methods]
impl URLSearchParams {
    #[qjs(constructor)]
    pub fn new<'js>(ctx: Ctx<'js>, init: Option<Value<'js>>) -> Result<Self> {
        let mut params = Vec::new();
        if let Some(val) = init {
            if let Some(s) = val.as_string() {
                let query = s.to_string();
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
        self.params.iter().find(|(k, _)| k == &name).map(|(_, v)| v.clone())
    }

    pub fn set(&mut self, name: String, value: String) {
        if let Some(pos) = self.params.iter().position(|(k, _)| k == &name) {
            self.params[pos].1 = value;
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

    pub fn to_string(&self) -> String {
        self.params.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&")
    }
}

pub fn register(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    globals.set("URLSearchParams", rquickjs::Class::<URLSearchParams>::register(ctx.clone())?)?;
    
    // Base64
    globals.set("btoa", rquickjs::Function::new(ctx.clone(), |s: String| -> String {
        base64::encode(s)
    }))?;
    
    globals.set("atob", rquickjs::Function::new(ctx.clone(), |s: String| -> Result<String> {
        base64::decode(s)
            .map(|b| String::from_utf8_lossy(&b).to_string())
            .map_err(|_| rquickjs::Error::new_from_js("Invalid base64", "Error"))
    }))?;

    Ok(())
}
