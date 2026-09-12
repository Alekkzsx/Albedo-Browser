//! # Orquestrador de Recursos de Rede (`ResourceFetcher`)
//!
//! Ponto de entrada central do subsistema de rede do Albedo Browser.
//! Coordena a integração completa entre:
//! - Resolução de Referrer e políticas de segurança
//! - Interceptação de Cache RFC 9111 (Cache Hits e Revalidação 304)
//! - Despacho de transporte assíncrono seguro com ALPN (HTTP/1.1 e HTTP/2)
//! - Descompressão transparente de conteúdo (`gzip`, `deflate`, `br`)
//! - Cancelamento granular atômico por `RequestId` / `CancellationToken`
//! - Registro e persistência de serviços alternativos `Alt-Svc` (RFC 7838)
//! - Pre-warming de conexões e Resource Hints (`dns-prefetch`, `preconnect`)
//! - Gerenciamento normativo de cookies RFC 6265bis e particionamento CHIPS
//! - HTTP Strict Transport Security (HSTS - RFC 6797) com auto-upgrade
//! - Purga automatizada de estado via `Clear-Site-Data` (RFC 8879)
//! - Suporte a requisições de faixa (`Range`) e `206 Partial Content`

use crate::alt_svc::{parse_alt_svc, AltSvcRegistry};
use crate::cache::entry::CacheEntry;
use crate::cache::storage::HttpCache;
use crate::cancel::CancellationRegistry;
use crate::clear_site_data::ClearSiteDataAction;
use crate::cookie::CookieJar;
use crate::encoding::extract_charset_from_content_type;
use crate::error::{NetError, NetResult};
use crate::fetch_metadata::SecFetchSite;
use crate::hsts::HstsStore;
use crate::priority::PriorityLevel;
use crate::redirect::{handle_redirect, RedirectAction};
use crate::request::{Request, RequestDestination, TryIntoUrl};
use crate::response::{Response, ResponseBody, ResponseTiming};
use crate::transport::TransportClient;
use ace_core::id::RequestId;
use ace_core::security::origin::Origin;
use ace_core::security::referrer::compute_referrer;
use http::header::{HeaderValue, CONTENT_TYPE, REFERER, STRICT_TRANSPORT_SECURITY};
use http::{Method, StatusCode};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::SystemTime;
use url::Url;
use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::config::{ResolverConfig, ResolverOpts};

/// Guarda RAII para remoção automática da requisição da tabela de cancelamento ao concluir.
struct RequestRegistrationGuard<'a>(&'a CancellationRegistry, RequestId);

impl<'a> Drop for RequestRegistrationGuard<'a> {
    fn drop(&mut self) {
        self.0.unregister(self.1);
    }
}

/// Orquestrador de requisições e busca de recursos da engine.
#[derive(Clone)]
pub struct ResourceFetcher {
    transport: TransportClient,
    cache: Arc<HttpCache>,
    cancellation_registry: Arc<CancellationRegistry>,
    alt_svc_registry: Arc<AltSvcRegistry>,
    cookie_jar: Arc<CookieJar>,
    hsts_store: Arc<HstsStore>,
    doh_resolver: Arc<TokioAsyncResolver>,
}

impl ResourceFetcher {
    /// Cria uma nova instância de `ResourceFetcher` com cache padrão de 64 MB.
    pub fn new() -> NetResult<Self> {
        Self::with_cache_capacity(64 * 1024 * 1024)
    }

    /// Cria uma nova instância com capacidade de cache personalizada em bytes.
    pub fn with_cache_capacity(cache_capacity_bytes: usize) -> NetResult<Self> {
        let transport = TransportClient::new()?;
        let cache = Arc::new(HttpCache::new(cache_capacity_bytes));
        let cancellation_registry = Arc::new(CancellationRegistry::new());
        let alt_svc_registry = Arc::new(AltSvcRegistry::new());
        let cookie_jar = Arc::new(CookieJar::new());
        let hsts_store = Arc::new(HstsStore::new());
        let doh_resolver = Arc::new(TokioAsyncResolver::tokio(
            ResolverConfig::cloudflare_https(),
            ResolverOpts::default(),
        ));

        Ok(Self {
            transport,
            cache,
            cancellation_registry,
            alt_svc_registry,
            cookie_jar,
            hsts_store,
            doh_resolver,
        })
    }

    /// Retorna uma referência compartilhada ao cache HTTP subjacente.
    pub fn cache(&self) -> &Arc<HttpCache> {
        &self.cache
    }

    /// Retorna uma referência ao registro de cancelamentos ativos.
    pub fn cancellation_registry(&self) -> &Arc<CancellationRegistry> {
        &self.cancellation_registry
    }

    /// Retorna uma referência ao registro de serviços alternativos Alt-Svc.
    pub fn alt_svc_registry(&self) -> &Arc<AltSvcRegistry> {
        &self.alt_svc_registry
    }

    /// Retorna uma referência ao pote de cookies do navegador.
    pub fn cookie_jar(&self) -> &Arc<CookieJar> {
        &self.cookie_jar
    }

    /// Retorna uma referência ao registro HSTS de domínios seguros.
    pub fn hsts_store(&self) -> &Arc<HstsStore> {
        &self.hsts_store
    }

    /// Cancela uma requisição em voo pelo seu `RequestId`.
    pub fn cancel(&self, id: RequestId) -> bool {
        self.cancellation_registry.cancel(id)
    }

    /// Executa pré-resolução DNS para um host em background (Resource Hint).
    pub async fn dns_prefetch(&self, host: &str) -> NetResult<()> {
        let _ = self.doh_resolver.lookup_ip(host)
            .await
            .map_err(|e| NetError::DnsResolutionFailed(host.into(), e.to_string()))?;
        Ok(())
    }

    /// Estabelece conexão TCP e TLS antecipada para uma origem (Resource Hint).
    pub async fn preconnect(&self, origin: &Origin) -> NetResult<()> {
        let url_str = origin.ascii_serialization();
        if let Ok(url) = Url::parse(&url_str) {
            if url.scheme() == "http" || url.scheme() == "https" {
                let req = Request::builder(url, Method::HEAD)?
                    .priority(PriorityLevel::Lowest)
                    .build();
                let _ = self.transport.execute(&req).await;
            }
        }
        Ok(())
    }

    /// Busca um sub-recurso especulativo identificado pelo `PreloadScanner`.
    pub async fn fetch_preload_hint(
        &self,
        url: impl TryIntoUrl,
        destination: RequestDestination,
    ) -> NetResult<Response> {
        let req = Request::get(url)?
            .destination(destination)
            .priority(destination.default_priority())
            .build();
        self.fetch(req).await
    }

    /// Executa uma requisição completa de recurso, orquestrando HSTS, cookies, cancelamento, cache, rede e redirecionamentos.
    pub async fn fetch(&self, mut req: Request) -> NetResult<Response> {
        let now = SystemTime::now();

        // 0. HSTS Auto-Upgrade: se a URL for http e o host exigir HSTS, reescreve em memória para https
        if let Some(upgraded_url) = self.hsts_store.upgrade_url(&req.url, now) {
            req.url = upgraded_url;
        }

        let nik = req.network_isolation_key.clone();

        // 1. Registra o token de cancelamento na tabela de requisições ativas
        self.cancellation_registry
            .register(req.id, req.cancellation_token.clone());
        let _guard = RequestRegistrationGuard(&self.cancellation_registry, req.id);

        if req.cancellation_token.is_cancelled() {
            return Err(NetError::Cancelled);
        }

        // 2. Injeção de Cookies relevantes (RFC 6265bis + CHIPS Partitioned)
        let top_level_site = nik.as_ref().and_then(|k| match &k.top_frame_origin {
            Origin::Tuple { host, .. } => Some(host.as_str().to_string()),
            Origin::Opaque(_) => None,
        });

        let is_same_site = req
            .initiator
            .as_ref()
            .map(|init| {
                let site = SecFetchSite::compute(Some(init), &req.url);
                site == SecFetchSite::SameOrigin || site == SecFetchSite::SameSite
            })
            .unwrap_or(true);

        if !req.headers.contains_key(http::header::COOKIE) {
            let is_nav_get = req.destination == RequestDestination::Document && req.method == Method::GET;
            if let Some(cookie_hdr) = self.cookie_jar.build_cookie_header(
                &req.url,
                top_level_site.as_deref(),
                req.credentials,
                is_same_site,
                is_nav_get,
                now,
            ) {
                req.headers.insert(http::header::COOKIE, cookie_hdr);
            }
        }

        // 3. Invalidação de cache em métodos mutantes (RFC 9111 §4.4)
        if req.method == Method::POST || req.method == Method::PUT || req.method == Method::DELETE {
            self.cache.invalidate(&req.url);
        }

        // 4. Consulta ao Cache HTTP RFC 9111 (apenas para requisições idempotentes GET/HEAD)
        let mut cached_entry = None;
        if req.method == Method::GET || req.method == Method::HEAD {
            if let Some(entry) = self.cache.get(nik.as_ref(), &req.url).await {
                if entry.is_fresh(now) {
                    // Cache Hit completo! Zero latência de rede.
                    return Ok(Response {
                        url: req.url,
                        status: entry.status,
                        headers: entry.headers.clone(),
                        body: ResponseBody::Full(entry.body.clone()),
                        mime_type: entry
                            .headers
                            .get(CONTENT_TYPE)
                            .and_then(|v| v.to_str().ok())
                            .map(|s| s.split(';').next().unwrap_or("").trim().into())
                            .unwrap_or_else(|| "application/octet-stream".into()),
                        charset: entry
                            .headers
                            .get(CONTENT_TYPE)
                            .and_then(|v| v.to_str().ok())
                            .and_then(extract_charset_from_content_type),
                        content_encoding: None,
                        retry_after: None,
                        content_range: None,
                        from_cache: true,
                        timing: ResponseTiming::default(),
                    });
                } else {
                    // Entrada expirada (stale) - injeta cabeçalhos de revalidação condicional
                    let cond_headers = entry.conditional_headers();
                    for (k, v) in cond_headers {
                        if let Some(name) = k {
                            req.headers.insert(name, v);
                        }
                    }
                    cached_entry = Some(entry);
                }
            }
        }

        // 5. Resolução da política de Referrer
        if let Some(ref referrer_url) = req.referrer {
            let current_origin = Origin::parse(referrer_url.as_str()).unwrap_or_else(|_| Origin::new_opaque());
            if let Some(computed_ref) = compute_referrer(&current_origin, referrer_url.as_str(), req.url.as_str(), req.referrer_policy) {
                if let Ok(val) = HeaderValue::from_str(&computed_ref) {
                    req.headers.insert(REFERER, val);
                }
            }
        }

        // 6. Execução de transporte com suporte a redirecionamentos (3xx)
        let max_redirects = match req.redirect_policy {
            crate::request::RedirectPolicy::Follow(max) => max,
            crate::request::RedirectPolicy::Manual => 0,
            crate::request::RedirectPolicy::Error => 0,
        };

        let mut visited_urls = HashSet::new();
        visited_urls.insert(req.url.to_string());

        let mut current_req = req;
        let mut current_cached_entry = cached_entry;

        loop {
            let request_time = SystemTime::now();
            let network_response = self.transport.execute(&current_req).await?;
            let response_time = SystemTime::now();

            // Processa cabeçalhos Set-Cookie da resposta
            self.cookie_jar.process_response_headers(
                &current_req.url,
                &network_response.headers,
                top_level_site.as_deref(),
                response_time,
            );

            // Atualiza HSTS se em conexão HTTPS
            if current_req.url.scheme() == "https" {
                if let Some(hsts_val) = network_response.headers.get(STRICT_TRANSPORT_SECURITY) {
                    if let Some(host) = current_req.url.host_str() {
                        self.hsts_store.update_from_header(host, hsts_val, response_time);
                    }
                }
            }

            // Processa cabeçalho W3C Clear-Site-Data se em contexto seguro
            if current_req.url.scheme() == "https" {
                if let Some(csd_val) = network_response.headers.get("clear-site-data") {
                    let action = ClearSiteDataAction::parse(csd_val);
                    if let Some(host) = current_req.url.host_str() {
                        if action.clear_cache {
                            self.cache.invalidate(&current_req.url);
                        }
                        if action.clear_cookies {
                            self.cookie_jar.clear_for_domain(host);
                        }
                    }
                }
            }

            // Análise e armazenamento de Alt-Svc (RFC 7838) se presente
            if let Some(alt_svc_raw) = network_response.headers.get("alt-svc").and_then(|v| v.to_str().ok()) {
                let records = parse_alt_svc(alt_svc_raw, response_time);
                if let Some(host) = current_req.url.host_str() {
                    self.alt_svc_registry.insert(nik.as_ref(), host, records);
                }
            }

            // 7. Tratamento de Revalidação Condicional 304 Not Modified
            if network_response.status == StatusCode::NOT_MODIFIED {
                if let Some(mut stale_entry) = current_cached_entry {
                    let updated = self.cache.update_304(
                        nik.as_ref(),
                        &current_req.url,
                        &network_response.headers,
                        response_time,
                    );
                    let final_entry = updated.unwrap_or_else(|| {
                        stale_entry.update_from_304(&network_response.headers, response_time);
                        stale_entry
                    });

                    return Ok(Response {
                        url: current_req.url,
                        status: StatusCode::OK,
                        headers: final_entry.headers.clone(),
                        body: ResponseBody::Full(final_entry.body.clone()),
                        mime_type: final_entry
                            .headers
                            .get(CONTENT_TYPE)
                            .and_then(|v| v.to_str().ok())
                            .map(|s| s.split(';').next().unwrap_or("").trim().into())
                            .unwrap_or_else(|| "application/octet-stream".into()),
                        charset: final_entry
                            .headers
                            .get(CONTENT_TYPE)
                            .and_then(|v| v.to_str().ok())
                            .and_then(extract_charset_from_content_type),
                        content_encoding: None,
                        retry_after: None,
                        content_range: None,
                        from_cache: true,
                        timing: network_response.timing,
                    });
                }
            }

            // 8. Tratamento de Redirecionamentos (3xx)
            if max_redirects > 0 {
                let redirect_action = handle_redirect(
                    &current_req,
                    network_response.status,
                    &network_response.headers,
                    &mut visited_urls,
                    max_redirects,
                )?;

                if let RedirectAction::Follow(follow) = redirect_action {
                    // Prepara próxima iteração do salto de redirecionamento
                    current_req.url = follow.new_url;
                    current_req.method = follow.new_method;
                    current_req.body = follow.new_body;
                    current_req.headers = follow.new_headers;
                    current_cached_entry = self.cache.get(nik.as_ref(), &current_req.url).await;
                    continue;
                }
            }

            // 9. Armazena no Cache se a resposta for elegível
            if CacheEntry::is_cacheable(&current_req.method, network_response.status, &network_response.headers) {
                let entry = CacheEntry::new(
                    current_req.url.clone(),
                    network_response.status,
                    network_response.headers.clone(),
                    network_response.body.as_bytes().to_vec().into(),
                    request_time,
                    response_time,
                );
                self.cache.put(nik.as_ref(), current_req.url.clone(), entry);
            }

            return Ok(network_response);
        }
    }

    /// Método ergonômico para buscar uma URL com prioridade especificada.
    pub async fn fetch_url(&self, url: impl TryIntoUrl, priority: PriorityLevel) -> NetResult<Response> {
        let req = Request::get(url)?.priority(priority).build();
        self.fetch(req).await
    }

    /// Método ergonômico para buscar o documento HTML principal com prioridade máxima.
    pub async fn fetch_document(&self, url: impl TryIntoUrl) -> NetResult<Response> {
        let req = Request::get(url)?
            .destination(RequestDestination::Document)
            .priority(PriorityLevel::VeryHigh)
            .build();
        self.fetch(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetcher_data_url_direct_hit() {
        let fetcher = ResourceFetcher::new().unwrap();
        let resp = fetcher
            .fetch_url("data:text/html,<h1>Test</h1>", PriorityLevel::VeryHigh)
            .await
            .unwrap();

        assert_eq!(resp.status, StatusCode::OK);
        assert_eq!(resp.mime_type.as_str(), "text/html");
        assert_eq!(resp.text().unwrap(), "<h1>Test</h1>");
    }

    #[tokio::test]
    async fn test_fetcher_cancellation() {
        let fetcher = ResourceFetcher::new().unwrap();
        let req_builder = Request::get("data:text/plain,slow").unwrap();
        let req = req_builder.build();

        req.cancellation_token.cancel();
        let result = fetcher.fetch(req).await;

        assert!(matches!(result, Err(NetError::Cancelled)));
    }
}
