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


// O SERVIÇO PRINCIPAL
pub struct FetchClient {
    client: reqwest::blocking::Client,
    _http3_client: Option<Http3Client>,
}

impl FetchClient {
    /// TODO: add docs
    pub fn new() -> Self {
        // HTTP/2 com fallback automático para HTTP/1.1
        // Benefícios de Performance:
        //   ✅ Multiplexing: múltiplas requisições na mesma conexão TCP
        //   ✅ Header compression (HPACK): reduz overhead de headers
        //   ✅ Server push: servidor envia recursos antecipadamente
        //   ✅ Stream prioritization: controlar ordem de carregamento
        //   ✅ Compatibilidade: fallback automático para HTTP/1.1 se necessário
        //
        // HTTP/3 também disponível (QUIC-based):
        //   ✅ 25% mais rápido em conexões frias (0-RTT resumption)
        //   ✅ Connection migration: transições WiFi↔Cellular sem reconectar
        //   ✅ Sem head-of-line blocking entre streams
        //   ✅ Melhor recuperação de perda de pacotes
        //
        // Impacto esperado:
        //   - Redução de ~50% em latência para múltiplos recursos (HTTP/2)
        //   - Redução de ~75% em latência para conexões novas (HTTP/3)
        //   - Melhor utilização de conexão TCP
        //   - Carregamento mais rápido de páginas com muitos assets
        let client = reqwest::blocking::Client::builder()
            .user_agent("Albedo/1.0 (Compatible; Rust Native)")
            // TCP connection pooling otimizado
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10) // Aumentado para melhor paralelismo
            .tcp_keepalive(Duration::from_secs(60)) // Manter conexões vivas por mais tempo
            // Configurações de timeout
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            // HTTP/2 com conhecimento prévio (não precisa de upgrade header)
            .http2_prior_knowledge()
            // Fallback automático para HTTP/1.1 se servidor não suporta HTTP/2
            .build()
            .unwrap_or_default();

        // Inicializar HTTP/3 client (fallback automático se falhar)
        let http3_client = Http3Client::new().ok();

        Self {
            client,
            _http3_client: http3_client,
        }
    }

    /// Executa o fetch de forma síncrona (no MVP blocking, no futuro Async)
    pub fn fetch(
        &self,
        url: &str,
        options: Option<FetchOptions>,
        parent_origin: Option<Origin>,
    ) -> Result<FetchResponse, FetchError> {
        let opts = options.unwrap_or_default();

        let target_origin = Origin::from_url(url);
        let is_cross_origin = if let (Some(parent), Some(target)) = (&parent_origin, &target_origin)
        {
            !parent.is_same_origin(target)
        } else {
            false
        };

        // Regra Rigorosa: Bloquear na largada se for cross-origin em modo strict same-origin
        if is_cross_origin && opts.mode == FetchMode::SameOrigin {
            tracing::warn!(?parent_origin, url = %url, "Same-Origin Policy blocked");
            return Err(FetchError::SameOriginBlocked);
        }

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
        let mut builder = self
            .client
            .request(method, url)
            .timeout(Duration::from_millis(opts.timeout_ms));

        // Injetar Header Origin agressivamente se for requests cross-origin (em modo Cors)
        if is_cross_origin && opts.mode == FetchMode::Cors {
            if let Some(parent) = &parent_origin {
                builder = builder.header("Origin", parent.to_string());
            }
        }

        // 3. Injetar Headers
        let mut header_map = HeaderMap::new();
        for (k, v) in opts.headers {
            if let (Ok(k_name), Ok(v_val)) = (
                HeaderName::from_bytes(k.as_bytes()),
                HeaderValue::from_str(&v),
            ) {
                header_map.insert(k_name, v_val);
            }
        }
        builder = builder.headers(header_map);

        // 4. Injetar Body (Se houver)
        if let Some(body_content) = opts.body {
            builder = builder.body(body_content);
        }

        // 5. Enviar e Processar
        tracing::debug!(
            method = %opts.method,
            url = %url,
            origin = ?parent_origin.as_ref().map(|o| o.to_string()),
            "Sending fetch request"
        );
        match builder.send() {
            Ok(resp) => {
                let status = resp.status();
                let final_url = resp.url().to_string();

                // Converter headers de volta para HashMap
                let mut resp_headers = HashMap::new();
                for (k, v) in resp.headers() {
                    resp_headers.insert(k.to_string(), v.to_str().unwrap_or("").to_string());
                }

                // VALIDAÇÃO IMPLACÁVEL DE CORS AQUI
                let mut opaque = false;

                if is_cross_origin {
                    if opts.mode == FetchMode::Cors {
                        let access_control = AccessControl::new();
                        // Precisamos checar se é Válido. Se não for, ERRO OPACA DE REDE (Bloqueia leitura).
                        // O origin da request
                        if let Some(parent) = &parent_origin {
                            if !access_control.validate_cors(parent, url, &resp_headers) {
                                tracing::warn!(url = %url, origin = %parent, "CORS denied by API");
                                return Err(FetchError::CorsBlocked);
                            }
                        }
                    } else if opts.mode == FetchMode::NoCors {
                        // Passa e baixa, mas a resposta DEVE ser marcada como Opaque. Javascript lá no fundo não lê.
                        opaque = true;
                    }
                }

                // Baixar corpo
                let body_bytes = resp.bytes().map_err(|_| FetchError::BodyError)?.to_vec();

                Ok(FetchResponse {
                    status: if opaque { 0 } else { status.as_u16() }, // Opaque responses mostram status 0
                    status_text: if opaque {
                        "".to_string()
                    } else {
                        status.canonical_reason().unwrap_or("Unknown").to_string()
                    },
                    headers: if opaque { HashMap::new() } else { resp_headers }, // Esconde headers se opaque
                    body_bytes,
                    url: final_url,
                    opaque,
                })
            }
            Err(e) => Err(FetchError::Network(e)),
        }
    }
}
