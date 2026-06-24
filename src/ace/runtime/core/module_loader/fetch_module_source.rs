use super::*;
//! ES Module Loader & Resolver para o Albedo Browser
//!
//! Implementa as traits `rquickjs::loader::Resolver` e `rquickjs::loader::Loader`
//! para suportar `<script type="module">`, `import`/`export` e `import()` dinâmico.
//!
//! ## Arquitetura
//! - `AlbedoModuleResolver`: Resolve specifiers de módulos contra a URL base da página.
//!   Suporta URLs absolutas, relativas (`./`, `../`), bare specifiers (`lodash`), e
//!   data URIs.
//! - `AlbedoModuleLoader`: Busca o código-fonte dos módulos. Primeiro verifica o
//!   cache interno (`module_cache`), depois faz download HTTP síncrono.
//! - `ModuleRegistry`: Cache compartilhado que armazena (url → código_fonte) para
//!   evitar re-downloads e re-avaliações.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Cache compartilhado de módulos já baixados/registrados.
/// Chave: URL absolutizada do módulo.
/// Valor: Código-fonte JS do módulo.


/// Faz download síncrono de um módulo JS via HTTP(S).
/// Retorna `None` se o download falhar.
pub(crate) fn fetch_module_source(url: &str) -> Option<String> {
    tracing::info!(url = %url, "Fetching module");

    // Data URI handling
    if url.starts_with("data:") {
        if let Some(comma_pos) = url.find(',') {
            let data = &url[comma_pos + 1..];
            // Check if base64 encoded
            let prefix = &url[..comma_pos];
            if prefix.contains(";base64") {
                if let Ok(decoded) = crate::utils::base64::decode(data) {
                    return String::from_utf8(decoded).ok();
                }
            } else {
                return Some(crate::ace::url::percent_encoding::decode(data));
            }
        }
        return None;
    }

    // HTTP fetch blocking
    match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Albedo/1.0")
        .build()
    {
        Ok(client) => match client.get(url).send() {
            Ok(response) => {
                if response.status().is_success() {
                    match response.text() {
                        Ok(text) => {
                            tracing::info!(url = %url, len = text.len(), "Fetched module");
                            Some(text)
                        }
                        Err(e) => {
                            tracing::error!(url = %url, ?e, "Failed to read response body");
                            None
                        }
                    }
                } else {
                    tracing::error!(status = %response.status(), url = %url, "HTTP error for module");
                    None
                }
            }
            Err(e) => {
                tracing::error!(url = %url, ?e, "Network error fetching module");
                None
            }
        },
        Err(e) => {
            tracing::error!(?e, "Failed to create HTTP client");
            None
        }
    }
}
