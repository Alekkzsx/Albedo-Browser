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


// A Resposta Fetch (Abstração da Resposta HTTP)
#[derive(Debug, Clone)]
pub struct FetchResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body_bytes: Vec<u8>, // Mantemos cru para suportar Imagem ou Texto
    pub url: String,
    pub opaque: bool, // OPAQUE (Bloqueia leitura por código web cross-origin sem CORS)
}

impl FetchResponse {
    // Retorna o corpo como String (Text) - Fails if opaque e cross origin (Simulamos devolvendo vazio para a engine)
    pub fn text(&self) -> String {
        if self.opaque {
            return "".to_string();
        }
        String::from_utf8_lossy(&self.body_bytes).to_string()
    }

    // Tenta retornar o corpo como JSON
    pub fn json(&self) -> Result<JsonValue, String> {
        if self.opaque {
            return Ok(JsonValue::Object(HashMap::new()));
        }
        json::parse(&self.text()).map_err(|e| format!("{:?}", e))
    }

    // Retorna se deu sucesso (200-299)
    pub fn ok(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}
