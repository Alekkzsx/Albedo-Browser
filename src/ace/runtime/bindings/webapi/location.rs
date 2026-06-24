use rquickjs::{Class, Context, Result};
use std::sync::{Arc, Mutex};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Location {
    url: String,
    #[qjs(skip_trace)]
    pending_navigation: Arc<Mutex<Option<String>>>,
}

impl Location {
    /// TODO: add docs
    pub fn new(url: String, pending_navigation: Arc<Mutex<Option<String>>>) -> Self {
        Self {
            url,
            pending_navigation,
        }
    }
}

#[rquickjs::methods]
impl Location {
    #[qjs(get, rename = "href")]
    pub fn href(&self) -> String {
        self.url.clone()
    }

    #[qjs(set, rename = "href")]
    pub fn set_href(&mut self, val: String) {
        tracing::debug!(href = %val, "Location.href set");
        *self.pending_navigation.lock().unwrap_or_else(|e| e.into_inner()) = Some(val.clone());
        self.url = val;
    }

    #[qjs(get)]
    pub fn protocol(&self) -> String {
        if let Some(pos) = self.url.find("://") {
            format!("{}:", &self.url[..pos])
        } else {
            "http:".to_string()
        }
    }

    #[qjs(get)]
    pub fn host(&self) -> String {
        // Simplified parsing
        if let Some(rest) = self.url.split("://").nth(1) {
            if let Some(end) = rest.find('/') {
                return rest[0..end].to_string();
            }
            return rest.to_string();
        }
        "".to_string()
    }

    #[qjs(get)]
    pub fn hostname(&self) -> String {
        let host = self.host();
        if let Some(pos) = host.find(':') {
            return host[0..pos].to_string();
        }
        host
    }

    #[qjs(get)]
    pub fn port(&self) -> String {
        let host = self.host();
        if let Some(pos) = host.find(':') {
            return host[pos + 1..].to_string();
        }
        "".to_string()
    }

    #[qjs(get)]
    pub fn pathname(&self) -> String {
        if let Some(rest) = self.url.split("://").nth(1) {
            if let Some(start) = rest.find('/') {
                if let Some(end) = rest[start..].find('?') {
                    return rest[start..start + end].to_string();
                }
                if let Some(end) = rest[start..].find('#') {
                    return rest[start..start + end].to_string();
                }
                return rest[start..].to_string();
            }
        }
        "/".to_string()
    }

    #[qjs(get)]
    pub fn search(&self) -> String {
        if let Some(pos) = self.url.find('?') {
            if let Some(end) = self.url[pos..].find('#') {
                return self.url[pos..pos + end].to_string();
            }
            return self.url[pos..].to_string();
        }
        "".to_string()
    }

    #[qjs(get)]
    pub fn hash(&self) -> String {
        if let Some(pos) = self.url.find('#') {
            return self.url[pos..].to_string();
        }
        "".to_string()
    }

    #[qjs(get)]
    pub fn origin(&self) -> String {
        if let Some(pos) = self.url.find("://") {
            if let Some(end) = self.url[pos + 3..].find('/') {
                return self.url[..pos + 3 + end].to_string();
            }
            return self.url.clone();
        }
        "".to_string()
    }

    /// TODO: add docs
    pub fn reload(&self) {
        tracing::debug!("Location.reload called");
    }

    /// TODO: add docs
    pub fn replace(&mut self, url: String) {
        tracing::debug!(url = %url, "Location.replace called");
        *self.pending_navigation.lock().unwrap_or_else(|e| e.into_inner()) = Some(url.clone());
        self.url = url;
    }

    /// TODO: add docs
    pub fn assign(&mut self, url: String) {
        tracing::debug!(url = %url, "Location.assign called");
        *self.pending_navigation.lock().unwrap_or_else(|e| e.into_inner()) = Some(url.clone());
        self.url = url;
    }
}

/// TODO: add docs
pub fn register(
    ctx: &Context,
    initial_url: &str,
    pending_navigation: Arc<Mutex<Option<String>>>,
) -> Result<()> {
    ctx.with(|ctx| {
        let global = ctx.globals();
        let location = Class::instance(
            ctx.clone(),
            Location::new(initial_url.to_string(), pending_navigation),
        )?;
        global.set("location", location)?;
        Ok(())
    })
}
