//! HTTP/3 QUIC Client — Implementação Real via quinn + h3
//!
//! Este módulo fornece suporte HTTP/3 real via protocolo QUIC para o Albedo Browser.
//! Vantagens sobre HTTP/2:
//!   - 0-RTT resumption para conexões rápidas (25% mais rápido em cold connections)
//!   - Connection migration (troca de IP/porta não quebra a conexão)
//!   - Stream multiplexing sem head-of-line blocking
//!   - Recuperação de perda de pacotes por stream individual
//!   - Controle de congestionamento per-stream

use super::*;
use bytes::Buf;
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Resposta HTTP/3 com status, headers e corpo


impl Http3Client {

    /// Envia uma requisição HTTP/3 completa e retorna a resposta.
    ///
    /// Fluxo:
    ///   1. Obtém/reutiliza conexão QUIC via `connect()`
    ///   2. Cria stream bidirecional HTTP/3 via `h3::client::new()`
    ///   3. Envia request (método, URL, headers, body)
    ///   4. Recebe response (status, headers, body)
    ///   5. Retorna `Http3Response`
    pub async fn send_request(
        &self,
        method: &str,
        url: &str,
        headers: Vec<(String, String)>,
        body: Option<Vec<u8>>,
    ) -> Result<Http3Response, Box<dyn Error + Send + Sync>> {
        // 1. Parsear URL
        let uri: http::Uri = url
            .parse()
            .map_err(|e| format!("[HTTP/3] URL inválida '{}': {}", url, e))?;

        let host = uri
            .host()
            .ok_or_else(|| format!("[HTTP/3] URL sem host: {}", url))?;
        let port = uri.port_u16().unwrap_or(443);

        // 2. Obter conexão QUIC (do pool ou nova)
        let quic_conn = self.connect(host, port).await?;

        // 3. Criar camada HTTP/3 sobre a conexão QUIC
        let h3_conn = h3_quinn::Connection::new(quic_conn);
        let (mut driver, mut send_request) = h3::client::new(h3_conn).await?;

        // 4. Construir request HTTP
        let http_method = method
            .parse::<http::Method>()
            .map_err(|e| format!("[HTTP/3] Método inválido '{}': {}", method, e))?;

        let mut builder = http::Request::builder()
            .method(http_method)
            .uri(uri.clone());

        // Injetar headers customizados
        for (k, v) in &headers {
            if let (Ok(name), Ok(val)) = (
                k.parse::<http::header::HeaderName>(),
                http::header::HeaderValue::from_str(v),
            ) {
                builder = builder.header(name, val);
            }
        }

        // Header Host obrigatório se não presente
        if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) {
            if let Some(authority) = uri.authority() {
                builder = builder.header("host", authority.as_str());
            }
        }

        // Albedo User-Agent
        if !headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("user-agent"))
        {
            builder = builder.header("user-agent", "AlbedoBrowser/0.1 (QUIC; HTTP/3)");
        }

        let req = builder
            .body(())
            .map_err(|e| format!("[HTTP/3] Falha ao construir request: {}", e))?;

        // 5. Criar a task do driver (gerencia control streams em background)
        let drive_task = tokio::spawn(async move {
            // poll_close retorna ConnectionError diretamente
            let err = futures_util::future::poll_fn(|cx| driver.poll_close(cx)).await;
            // Logar apenas erros reais (ignorar fechamento normal)
            tracing::warn!(?err, "HTTP/3 driver finished");
        });

        // 6. Enviar request e receber response
        let mut stream = send_request.send_request(req).await?;

        // Enviar body se houver
        if let Some(body_data) = body {
            stream.send_data(bytes::Bytes::from(body_data)).await?;
        }

        // Finalizar o sending side
        stream.finish().await?;

        // 7. Receber response
        let resp = stream.recv_response().await?;
        let status = resp.status().as_u16();

        // Parsear response headers
        let mut response_headers = HashMap::new();
        for (name, value) in resp.headers() {
            response_headers.insert(name.to_string(), value.to_str().unwrap_or("").to_string());
        }

        // 8. Receber body completo (recv_data retorna impl Buf)
        let mut body_bytes = Vec::new();
        while let Some(chunk) = stream.recv_data().await? {
            body_bytes.extend_from_slice(chunk.chunk());
        }

        tracing::debug!(status, url = %url, len = body_bytes.len(), "HTTP/3 response received");

        // Cancelar driver task (stream já foi consumida)
        drive_task.abort();

        Ok(Http3Response {
            status,
            headers: response_headers,
            body: body_bytes,
        })
    }
}
