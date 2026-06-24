use super::*;
// ARQUIVO: src/net/fetch.rs

use crate::shared::json::{self, JsonValue};
use crate::network::http3::Http3Client;
use crate::network::security::{AccessControl, Origin};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

// Erros Específicos do Fetch


// Configuração da Requisição (Espelha o objeto 'init' do JS)
#[derive(Debug, Clone)]
pub struct FetchOptions {
    pub method: String, // GET, POST, PUT...
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub mode: FetchMode,
    pub timeout_ms: u64,
}

impl FetchOptions {
    /// TODO: add docs
    pub fn to_json(&self) -> JsonValue {
        let mut map = HashMap::new();
        map.insert("method".to_string(), JsonValue::String(self.method.clone()));

        let mut headers_map = HashMap::new();
        for (k, v) in &self.headers {
            headers_map.insert(k.clone(), JsonValue::String(v.clone()));
        }
        map.insert("headers".to_string(), JsonValue::Object(headers_map));

        map.insert(
            "body".to_string(),
            match &self.body {
                Some(b) => JsonValue::String(b.clone()),
                None => JsonValue::Null,
            },
        );

        map.insert("mode".to_string(), self.mode.to_json());
        map.insert(
            "timeout_ms".to_string(),
            JsonValue::Number(self.timeout_ms as f64),
        );

        JsonValue::Object(map)
    }

    /// TODO: add docs
    pub fn from_json(value: &JsonValue) -> Option<Self> {
        let obj = value.as_object()?;

        let method = obj.get("method")?.as_string()?.to_string();

        let mut headers = HashMap::new();
        if let Some(h_val) = obj.get("headers") {
            if let Some(h_obj) = h_val.as_object() {
                for (k, v) in h_obj {
                    if let Some(s) = v.as_string() {
                        headers.insert(k.clone(), s.to_string());
                    }
                }
            }
        }

        let body = obj
            .get("body")
            .and_then(|v| v.as_string().map(|s| s.to_string()));
        let mode = obj
            .get("mode")
            .and_then(|v| FetchMode::from_json(v))
            .unwrap_or_default();
        let timeout_ms = obj
            .get("timeout_ms")
            .and_then(|v| v.as_number())
            .map(|n| n as u64)
            .unwrap_or(10_000);

        Some(Self {
            method,
            headers,
            body,
            mode,
            timeout_ms,
        })
    }
}

// Padrões do objeto fetch
impl Default for FetchOptions {
pub(crate) fn default() -> Self {
        Self {
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
            mode: FetchMode::Cors,
            timeout_ms: 10_000, // 10 segundos timeout padrão
        }
    }
}
