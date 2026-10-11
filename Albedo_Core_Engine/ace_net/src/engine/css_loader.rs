//! # Carregador Assíncrono de Folhas de Estilo CSS (`CssLoaderService`)
//!
//! Integra o carregamento de CSS externo (`<link rel="stylesheet">` e `@import`)
//! com prioridade crítica (RFC 9218) e cache semântico HTTP (RFC 9111).

use crate::cache::storage::HttpCache;
use crate::engine::fetcher::ResourceFetcher;
use crate::engine::priority::PriorityLevel;
use crate::error::NetError;
use std::sync::Arc;
use std::time::SystemTime;
use url::Url;

/// Serviço de carregamento assíncrono de CSS para o motor de renderização.
#[derive(Clone)]
pub struct CssLoaderService {
    fetcher: Arc<ResourceFetcher>,
    cache: Option<Arc<HttpCache>>,
}

impl CssLoaderService {
    /// Cria uma nova instância com o `ResourceFetcher` configurado.
    pub fn new(fetcher: Arc<ResourceFetcher>) -> Self {
        Self {
            fetcher,
            cache: None,
        }
    }

    /// Anexa o motor de cache RFC 9111 para resolução imediata sem requisições de rede.
    pub fn with_cache(mut self, cache: Arc<HttpCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Carrega o conteúdo de uma folha de estilo por URL.
    pub async fn load_stylesheet(&self, url_str: &str) -> Result<String, NetError> {
        // 1. Consulta ao cache semântico RFC 9111 (se configurado)
        if let Some(ref cache) = self.cache {
            if let Ok(parsed_url) = Url::parse(url_str) {
                if let Some(cached_resp) = cache.get(None, &parsed_url).await {
                    if cached_resp.is_fresh(SystemTime::now()) {
                        return Ok(String::from_utf8_lossy(&cached_resp.body).to_string());
                    }
                }
            }
        }

        // 2. Requisição HTTP com prioridade alta
        let resp = self.fetcher.fetch_url(url_str, PriorityLevel::High).await?;

        if !resp.status.is_success() {
            return Err(NetError::HttpProtocolError(format!(
                "Falha ao carregar folha de estilo {}: HTTP {}",
                url_str, resp.status
            )));
        }

        resp.text_async().await
    }
}
