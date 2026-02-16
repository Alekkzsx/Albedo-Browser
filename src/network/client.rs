// ARQUIVO: src/net/fetch.rs

use std::collections::HashMap;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

// Erros Específicos do Fetch
#[derive(thiserror::Error, Debug)]
pub enum FetchError {
    #[error("Erro de Rede: {0}")]
    Network(#[from] reqwest::Error),
    #[error("URL Inválida: {0}")]
    InvalidUrl(String),
    #[error("Método HTTP Inválido")]
    InvalidMethod,
    #[error("Falha ao processar corpo")]
    BodyError,
}

// Configuração da Requisição (Espelha o objeto 'init' do JS)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchOptions {
    pub method: String, // GET, POST, PUT...
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub mode: String,   // cors, no-cors (por enquanto placeholder)
    pub timeout_ms: u64,
}

// Padrões do objeto fetch
impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
            mode: "cors".to_string(),
            timeout_ms: 10_000, // 10 segundos timeout padrão
        }
    }
}

// A Resposta Fetch (Abstração da Resposta HTTP)
#[derive(Debug, Clone)]
pub struct FetchResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body_bytes: Vec<u8>, // Mantemos cru para suportar Imagem ou Texto
    pub url: String,
}

impl FetchResponse {
    // Retorna o corpo como String (Text)
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body_bytes).to_string()
    }

    // Tenta retornar o corpo como JSON
    pub fn json(&self) -> Result<Value, serde_json::Error> {
        serde_json::from_slice(&self.body_bytes)
    }

    // Retorna se deu sucesso (200-299)
    pub fn ok(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}

// O SERVIÇO PRINCIPAL
pub struct FetchClient {
    client: reqwest::blocking::Client,
}

impl FetchClient {
    pub fn new() -> Self {
        let client = reqwest::blocking::Client::builder()
            .user_agent("Albedo/1.0 (Compatible; Rust Native)")
            .pool_idle_timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();

        Self { client }
    }

    /// Executa o fetch de forma síncrona (no MVP blocking, no futuro Async)
    pub fn fetch(&self, url: &str, options: Option<FetchOptions>) -> Result<FetchResponse, FetchError> {
        let opts = options.unwrap_or_default();
        
        // 1. Validar Método
        let method = match opts.method.to_uppercase().as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "HEAD" => reqwest::Method::HEAD,
            _ => return Err(FetchError::InvalidMethod),
        };

        // 2. Construir Requisição
        let mut builder = self.client.request(method, url)
            .timeout(Duration::from_millis(opts.timeout_ms));

        // 3. Injetar Headers
        let mut header_map = HeaderMap::new();
        for (k, v) in opts.headers {
            if let (Ok(k_name), Ok(v_val)) = (HeaderName::from_bytes(k.as_bytes()), HeaderValue::from_str(&v)) {
                header_map.insert(k_name, v_val);
            }
        }
        builder = builder.headers(header_map);

        // 4. Injetar Body (Se houver)
        if let Some(body_content) = opts.body {
            builder = builder.body(body_content);
        }

        // 5. Enviar e Processar
        println!("FETCH: Enviando {} para {}", opts.method, url);
        match builder.send() {
            Ok(resp) => {
                let status = resp.status();
                let final_url = resp.url().to_string();
                
                // Converter headers de volta para HashMap
                let mut resp_headers = HashMap::new();
                for (k, v) in resp.headers() {
                    resp_headers.insert(k.to_string(), v.to_str().unwrap_or("").to_string());
                }

                // Baixar corpo
                let body_bytes = resp.bytes().map_err(|_| FetchError::BodyError)?.to_vec();

                Ok(FetchResponse {
                    status: status.as_u16(),
                    status_text: status.canonical_reason().unwrap_or("Unknown").to_string(),
                    headers: resp_headers,
                    body_bytes,
                    url: final_url,
                })
            },
            Err(e) => Err(FetchError::Network(e))
        }
    }
}