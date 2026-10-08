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

use crate::protocol::alt_svc::{parse_alt_svc, AltSvcRegistry};
use crate::cache::entry::CacheEntry;
use crate::cache::partition::NetworkIsolationKey;
use crate::cache::storage::HttpCache;
use crate::engine::cancel::CancellationRegistry;
use crate::security::clear_site_data::ClearSiteDataAction;
use crate::cookie::CookieJar;
use crate::http::encoding::extract_charset_from_content_type;
use crate::error::{NetError, NetResult};
use crate::security::fetch_metadata::SecFetchSite;
use crate::security::hsts::HstsStore;
use crate::engine::priority::PriorityLevel;
use crate::http::redirect::{handle_redirect, RedirectAction};
use crate::http::request::{Request, RequestDestination, TryIntoUrl};
use crate::http::response::{Response, ResponseBody, ResponseTiming};
use crate::engine::service_worker_hook::ServiceWorkerHook;
use crate::transport::TransportClient;
use ace_core::id::RequestId;
use ace_core::security::origin::Origin;
use ace_core::security::referrer::compute_referrer;
use http::header::{HeaderValue, CONTENT_TYPE, REFERER, STRICT_TRANSPORT_SECURITY};
use http::{Method, StatusCode};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
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
    service_worker_hook: Arc<parking_lot::RwLock<Option<Arc<dyn ServiceWorkerHook>>>>,
    metrics: Arc<crate::telemetry::metrics::FetcherMetrics>,
    scheduler: Arc<crate::engine::scheduler::ResourceScheduler>,
    cors_cache: Arc<crate::security::cors_cache::CorsCache>,
    disk_path: Option<std::path::PathBuf>,
    har_exporter: Arc<parking_lot::RwLock<Option<Arc<crate::telemetry::har::HarExporter>>>>,
    net_log_collector: Arc<crate::telemetry::net_log::NetLogCollector>,
    dictionary_manager: Arc<crate::http::dictionary::DictionaryManager>,
    proxy_config: Arc<parking_lot::RwLock<crate::transport::proxy::ProxyConfig>>,
    proxy_bypass_list: Arc<parking_lot::RwLock<crate::transport::proxy::ProxyBypassList>>,
}

impl ResourceFetcher {
    /// Cria uma nova instância de `ResourceFetcher` com cache padrão de 64 MB.
    pub fn new() -> NetResult<Self> {
        Self::with_cache_capacity(64 * 1024 * 1024)
    }

    /// Cria uma nova instância com capacidade de cache personalizada em bytes.
    pub fn with_cache_capacity(cache_capacity_bytes: usize) -> NetResult<Self> {
        Self::with_cache_and_disk(cache_capacity_bytes, None::<std::path::PathBuf>)
    }

    /// Cria uma nova instância com capacidade em memória e diretório opcional para cache persistente em disco (L2).
    pub fn with_cache_and_disk(cache_capacity_bytes: usize, disk_path: Option<impl Into<std::path::PathBuf>>) -> NetResult<Self> {
        let doh_resolver = Arc::new(TokioAsyncResolver::tokio(
            ResolverConfig::cloudflare_https(),
            ResolverOpts::default(),
        ));
        let transport = TransportClient::with_resolver(
            crate::transport::DohHappyEyeballsResolver::with_resolver(doh_resolver.clone())
        )?;
        let disk_buf: Option<std::path::PathBuf> = disk_path.map(|p| p.into());
        let mut http_cache = HttpCache::new(cache_capacity_bytes);
        if let Some(ref p) = disk_buf {
            http_cache = http_cache.with_disk_path(p.clone());
        }
        let cache = Arc::new(http_cache);
        let cancellation_registry = Arc::new(CancellationRegistry::new());
        let alt_svc_registry = Arc::new(AltSvcRegistry::new());
        let cookie_jar = Arc::new(CookieJar::new());
        let hsts_store = Arc::new(HstsStore::new());

        // Restauração de sessão persistente em disco (Cookies, HSTS e rotas Alt-Svc)
        if let Some(ref p) = disk_buf {
            let _ = cookie_jar.load_from_file(p.join("cookies.txt"));
            let _ = hsts_store.load_from_file(p.join("hsts.json"));
            let _ = alt_svc_registry.load_from_file(p.join("altsvc.json"));
        }

        let service_worker_hook = Arc::new(parking_lot::RwLock::new(None));
        let metrics = Arc::new(crate::telemetry::metrics::FetcherMetrics::new());
        let scheduler = crate::engine::scheduler::ResourceScheduler::new(crate::engine::scheduler::SchedulerConfig::default());
        let cors_cache = Arc::new(crate::security::cors_cache::CorsCache::new());
        let har_exporter = Arc::new(parking_lot::RwLock::new(None));
        let net_log_collector = Arc::new(crate::telemetry::net_log::NetLogCollector::new());
        let dictionary_manager = Arc::new(crate::http::dictionary::DictionaryManager::new());
        let proxy_config = Arc::new(parking_lot::RwLock::new(crate::transport::proxy::ProxyConfig::Direct));
        let proxy_bypass_list = Arc::new(parking_lot::RwLock::new(crate::transport::proxy::ProxyBypassList::new()));

        Ok(Self {
            transport,
            cache,
            cancellation_registry,
            alt_svc_registry,
            cookie_jar,
            hsts_store,
            doh_resolver,
            service_worker_hook,
            metrics,
            scheduler,
            cors_cache,
            disk_path: disk_buf,
            har_exporter,
            net_log_collector,
            dictionary_manager,
            proxy_config,
            proxy_bypass_list,
        })
    }

    /// Define a configuração global de proxy para a engine.
    pub fn set_proxy(&self, config: crate::transport::proxy::ProxyConfig) {
        *self.proxy_config.write() = config;
    }

    /// Retorna a configuração de proxy atual.
    pub fn proxy(&self) -> crate::transport::proxy::ProxyConfig {
        self.proxy_config.read().clone()
    }

    /// Define a lista de regras de bypass do proxy.
    pub fn set_proxy_bypass_list(&self, list: crate::transport::proxy::ProxyBypassList) {
        *self.proxy_bypass_list.write() = list;
    }

    /// Retorna a lista de regras de bypass do proxy.
    pub fn proxy_bypass_list(&self) -> crate::transport::proxy::ProxyBypassList {
        self.proxy_bypass_list.read().clone()
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

    /// Registra um gancho de Service Worker na engine de rede.
    pub fn set_service_worker_hook(&self, hook: Arc<dyn ServiceWorkerHook>) {
        let mut w = self.service_worker_hook.write();
        *w = Some(hook);
    }

    /// Retorna o gancho do Service Worker.
    pub fn service_worker_hook(&self) -> &Arc<parking_lot::RwLock<Option<Arc<dyn ServiceWorkerHook>>>> {
        &self.service_worker_hook
    }

    /// Remove o interceptador de Service Workers.
    pub fn clear_service_worker_hook(&self) {
        *self.service_worker_hook.write() = None;
    }

    /// Cancela uma requisição em voo pelo seu `RequestId`.
    pub fn cancel(&self, id: RequestId) -> bool {
        self.cancellation_registry.cancel(id)
    }

    /// Retorna o diretório em disco associado à sessão (se configurado).
    pub fn disk_path(&self) -> Option<&std::path::PathBuf> {
        self.disk_path.as_ref()
    }

    /// Salva de forma atômica no disco todo o estado de sessão (Cookies, HSTS e rotas Alt-Svc).
    pub fn save_session_to_disk(&self) -> NetResult<()> {
        if let Some(ref p) = self.disk_path {
            let _ = std::fs::create_dir_all(p);
            let _ = self.cookie_jar.save_to_file(p.join("cookies.txt"));
            let _ = self.hsts_store.save_to_file(p.join("hsts.json"));
            let _ = self.alt_svc_registry.save_to_file(p.join("altsvc.json"));
        }
        Ok(())
    }

    /// Executa pré-resolução DNS para um host em background (Resource Hint via DoH).
    pub async fn dns_prefetch(&self, host: &str) -> NetResult<()> {
        let clean = host.trim();
        if clean.is_empty() {
            return Ok(());
        }
        crate::telemetry::net_log::log_net_event(
            crate::telemetry::net_log::NetEventType::DnsStart,
            clean,
            "DNS prefetch iniciado via DoH",
        );
        let _ = self.doh_resolver.lookup_ip(clean)
            .await
            .map_err(|e| NetError::DnsResolutionFailed(clean.into(), e.to_string()))?;
        crate::telemetry::net_log::log_net_event(
            crate::telemetry::net_log::NetEventType::DnsEnd,
            clean,
            "DNS prefetch concluído",
        );
        Ok(())
    }

    /// Estabelece conexão TCP e TLS antecipada para uma origem (Resource Hint).
    pub async fn preconnect(&self, origin: &Origin) -> NetResult<()> {
        self.execute_preconnect(origin, None).await
    }

    /// Executa um preconnect especulativo particionado, respeitando NIK e PNA.
    pub async fn execute_preconnect(&self, origin: &Origin, nik: Option<&NetworkIsolationKey>) -> NetResult<()> {
        if let Origin::Tuple { host, .. } = origin {
            if let Some(nik_ref) = nik {
                if let Origin::Tuple { host: h, .. } = &nik_ref.top_frame_origin {
                    if !crate::security::pna::is_host_private_or_local(&h.as_str())
                        && crate::security::pna::is_host_private_or_local(&host.as_str())
                    {
                        crate::telemetry::net_log::log_net_event(
                            crate::telemetry::net_log::NetEventType::Warning,
                            &host.as_str(),
                            "Preconnect bloqueado por política PNA",
                        );
                        return Err(NetError::SecurityViolation("PNA: Preconnect para rede privada bloqueado".into()));
                    }
                }
            }
        }
        self.transport.preconnect(origin, nik).await
    }

    /// Despacha uma dica de recurso especulativa (Resource Hint) de forma unificada.
    pub async fn handle_resource_hint(&self, hint: crate::telemetry::hints::ResourceHint, nik: Option<&NetworkIsolationKey>) -> NetResult<()> {
        match hint {
            crate::telemetry::hints::ResourceHint::DnsPrefetch(host) => {
                self.dns_prefetch(&host).await
            }
            crate::telemetry::hints::ResourceHint::Preconnect { origin, .. } => {
                self.execute_preconnect(&origin, nik).await
            }
            crate::telemetry::hints::ResourceHint::Preload { url, destination } => {
                let mut builder = Request::get(url)?
                    .destination(destination)
                    .priority(destination.default_priority());
                if let Some(nik_ref) = nik {
                    builder = builder.network_isolation_key(nik_ref.clone());
                }
                let req = builder.build();
                let fetcher = self.clone();
                tokio::spawn(async move {
                    let _ = fetcher.fetch(req).await;
                });
                Ok(())
            }
        }
    }


    /// Retorna uma referência às métricas de rede
    pub fn metrics(&self) -> &Arc<crate::telemetry::metrics::FetcherMetrics> {
        &self.metrics
    }

    /// Ativa a gravação de tráfego de rede no formato HAR 1.2 persistindo em arquivo.
    pub fn enable_har_recording(&self, path: impl Into<std::path::PathBuf>) -> Arc<crate::telemetry::har::HarExporter> {
        let exporter = Arc::new(crate::telemetry::har::HarExporter::new(path.into()));
        *self.har_exporter.write() = Some(exporter.clone());
        exporter
    }

    /// Ativa a gravação de tráfego de rede no formato HAR 1.2 puramente em memória (ideal para DevTools).
    pub fn enable_in_memory_har_recording(&self) -> Arc<crate::telemetry::har::HarExporter> {
        let exporter = Arc::new(crate::telemetry::har::HarExporter::in_memory());
        *self.har_exporter.write() = Some(exporter.clone());
        exporter
    }

    /// Desativa a gravação de tráfego HAR.
    pub fn disable_har_recording(&self) {
        *self.har_exporter.write() = None;
    }

    /// Retorna o exportador de HAR ativo, se houver.
    pub fn har_exporter(&self) -> Option<Arc<crate::telemetry::har::HarExporter>> {
        self.har_exporter.read().clone()
    }

    /// Retorna o coletor Chromium NetLog da engine.
    pub fn net_log_collector(&self) -> &Arc<crate::telemetry::net_log::NetLogCollector> {
        &self.net_log_collector
    }

    /// Retorna uma referência ao gerenciador de dicionários de compressão (RFC 9290).
    pub fn dictionary_manager(&self) -> &Arc<crate::http::dictionary::DictionaryManager> {
        &self.dictionary_manager
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
    #[tracing::instrument(skip(self, req), fields(url = %req.url, method = %req.method, id = %req.id))]
    pub async fn fetch(&self, req: Request) -> NetResult<Response> {
        self.metrics.inc_total_requests();
        self.metrics.inc_in_flight();
        
        let req_id_raw = req.id.raw();
        let req_url_str = req.url.to_string();
        self.net_log_collector.record(
            req_id_raw,
            crate::telemetry::net_log::NetEventType::RequestStart,
            &req_url_str,
            &format!("Iniciando requisição {} {}", req.method, req.url),
        );

        let (priority_tx, priority_rx) = tokio::sync::watch::channel(req.priority);
        self.scheduler.register_active_stream(req.id, priority_tx);
        
        let req_id = req.id;
        let scheduler_clone = self.scheduler.clone();

        let har_exporter_opt = self.har_exporter.read().clone();
        let req_clone_for_har = if har_exporter_opt.is_some() {
            Some(req.clone())
        } else {
            None
        };

        let mut result = self.fetch_internal(req).await;
        
        self.metrics.dec_in_flight();
        match &mut result {
            Ok(resp) => {
                self.net_log_collector.record(
                    req_id_raw,
                    crate::telemetry::net_log::NetEventType::ResponseStart,
                    &req_url_str,
                    &format!("Resposta recebida com status {}", resp.status),
                );

                if let (Some(exporter), Some(req_orig)) = (har_exporter_opt, req_clone_for_har) {
                    let har_entry = crate::telemetry::har::HarEntry::from_response(&req_orig, resp);
                    exporter.record_entry(har_entry);
                }

                if matches!(resp.body, crate::http::response::ResponseBody::Stream(_)) {
                    if let Some(stream_box) = resp.body.take_stream().await {
                        let throttled = crate::engine::throttle::ThrottledStream::new(
                            stream_box,
                            priority_rx,
                            scheduler_clone,
                            req_id,
                        );
                        resp.body = crate::http::response::ResponseBody::Stream(std::sync::Arc::new(tokio::sync::Mutex::new(Some(Box::pin(throttled)))));
                    } else {
                        self.scheduler.unregister_active_stream(req_id);
                    }
                } else {
                    self.scheduler.unregister_active_stream(req_id);
                }
            }
            Err(NetError::Cancelled) => {
                self.scheduler.unregister_active_stream(req_id);
                self.metrics.cancelled_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.net_log_collector.record(
                    req_id_raw,
                    crate::telemetry::net_log::NetEventType::Cancel,
                    &req_url_str,
                    "Requisição cancelada",
                );
                crate::telemetry::net_log::log_net_event(crate::telemetry::net_log::NetEventType::Cancel, "", "Request cancelled");
            }
            Err(e) => {
                self.scheduler.unregister_active_stream(req_id);
                self.metrics.failed_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.net_log_collector.record(
                    req_id_raw,
                    crate::telemetry::net_log::NetEventType::Error,
                    &req_url_str,
                    &format!("Erro na requisição: {}", e),
                );
                crate::telemetry::net_log::log_net_error(crate::telemetry::net_log::NetEventType::Error, "", e);
            }
        }
        result
    }

    async fn fetch_internal(&self, mut req: Request) -> NetResult<Response> {
        let now = SystemTime::now();

        let nik = req.network_isolation_key.clone();

        // W3C Mixed Content Level 2: auto-upgrade passivo e bloqueio de ativo em contexto seguro
        if let Some(ref k) = nik {
            crate::security::mixed_content::check_and_apply_mixed_content(
                Some(&k.top_frame_origin),
                &mut req.url,
                req.destination,
                false,
            )?;
        }

        if let Some(sw_response) = self.apply_hsts_and_service_worker(&mut req, now).await? {
            return Ok(sw_response);
        }

        self.apply_security_policies(&req, nik.as_ref()).await?;

        // 1. Registra o token de cancelamento na tabela de requisições ativas
        self.cancellation_registry
            .register(req.id, req.cancellation_token.clone());
        let _guard = RequestRegistrationGuard(&self.cancellation_registry, req.id);

        if req.cancellation_token.is_cancelled() {
            return Err(NetError::Cancelled);
        }

        // 2. Injeção de Cookies relevantes
        self.inject_cookies(&mut req, nik.as_ref(), now);

        // 3. Invalidação de cache em métodos mutantes
        self.invalidate_cache_if_mutating(&req);

        // 4. Consulta ao Cache HTTP RFC 9111
        let (cached_entry, cached_resp) = self.check_http_cache(&mut req, nik.as_ref(), now).await?;
        if let Some(resp) = cached_resp {
            return Ok(resp);
        }

        // 5. Resolução da política de Referrer
        self.apply_referrer_policy(&mut req);

        // 5.5 Injeção de Compression Dictionary Transport (RFC 9290)
        self.apply_compression_dictionary_hints(&mut req, nik.as_ref(), now);

        // 6. Execução de transporte com suporte a redirecionamentos (3xx)
        self.execute_network_transport(req, nik.as_ref(), cached_entry, now).await
    }

    async fn apply_hsts_and_service_worker(&self, req: &mut Request, now: SystemTime) -> NetResult<Option<Response>> {
        crate::engine::pipeline::apply_hsts(req, self, now);
        crate::engine::pipeline::apply_service_worker(req, self).await
    }

    async fn apply_security_policies(&self, req: &Request, nik: Option<&crate::cache::NetworkIsolationKey>) -> NetResult<()> {
        // 0.15 Private Network Access (PNA) Nível 1
        if let Some(host) = req.url.host_str() {
            if crate::security::pna::is_host_private_or_local(host) {
                let initiator_is_public = nik.map_or(false, |n| {
                    if let ace_core::security::origin::Origin::Tuple { host: h, .. } = &n.top_frame_origin {
                        !crate::security::pna::is_host_private_or_local(&h.as_str())
                    } else {
                        false
                    }
                });
                
                if initiator_is_public {
                    crate::telemetry::net_log::log_net_event(
                        crate::telemetry::net_log::NetEventType::Warning,
                        req.url.as_str(),
                        "Bloqueado pelo PNA Nivel 1: hostname aponta para rede local ou privada",
                    );
                    return Err(NetError::SecurityViolation("Private Network Access bloqueado na origem (Nivel 1)".to_string()));
                }
            }
        }

        // 0.2 CORS Preflight
        if crate::security::cors::requires_preflight(req) {
            if !self.cors_cache.is_cached_and_valid(req) {
                let preflight_req = crate::security::cors::build_preflight_request(req)?;
                let preflight_resp = self.transport.execute(&preflight_req).await?;
                crate::security::cors::validate_cors_response(&preflight_req, &preflight_resp)?;
                
                if let Some(max_age_val) = preflight_resp.headers.get("access-control-max-age").and_then(|v| v.to_str().ok()) {
                    if let Ok(mut max_age_secs) = max_age_val.parse::<u64>() {
                        if max_age_secs > 7200 {
                            max_age_secs = 7200;
                        }
                        if max_age_secs > 0 {
                            let allow_methods = preflight_resp.headers.get("access-control-allow-methods")
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("")
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .collect();
                                
                            let allow_headers = preflight_resp.headers.get("access-control-allow-headers")
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("")
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .collect();
                                
                            let allow_credentials = preflight_resp.headers.get("access-control-allow-credentials")
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("") == "true";

                            let req_origin_str = req.headers.get(http::header::ORIGIN).and_then(|v| v.to_str().ok()).unwrap_or("");
                            
                            self.cors_cache.insert(req_origin_str, req.url.as_str(), max_age_secs, allow_methods, allow_headers, allow_credentials);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn inject_cookies(&self, req: &mut Request, nik: Option<&crate::cache::NetworkIsolationKey>, now: SystemTime) {
        let top_level_site = nik.and_then(|k| match &k.top_frame_origin {
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
    }

    fn invalidate_cache_if_mutating(&self, req: &Request) {
        if req.method == Method::POST || req.method == Method::PUT || req.method == Method::DELETE {
            self.cache.invalidate(&req.url);
        }
    }

    async fn check_http_cache(&self, req: &mut Request, nik: Option<&crate::cache::NetworkIsolationKey>, now: SystemTime) -> NetResult<(Option<CacheEntry>, Option<Response>)> {
        let mut cached_entry = None;
        if req.method == Method::GET || req.method == Method::HEAD {
            let mut is_range_request = false;
            let mut requested_start = 0;
            let mut requested_end = 0;

            if let Some(range_val) = req.headers.get(http::header::RANGE).and_then(|v| v.to_str().ok()) {
                if let Some(stripped) = range_val.strip_prefix("bytes=") {
                    let parts: Vec<&str> = stripped.split('-').collect();
                    if parts.len() == 2 {
                        if let (Ok(s), Ok(e)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                            is_range_request = true;
                            requested_start = s;
                            requested_end = e;
                        }
                    }
                }
            }

            if is_range_request {
                if let Some(bytes) = self.cache.get_range(nik, &req.url, requested_start, requested_end).await {
                    self.metrics.cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    self.metrics.bytes_served_from_cache.fetch_add(
                        bytes.len() as u64,
                        std::sync::atomic::Ordering::Relaxed,
                    );
                    crate::telemetry::net_log::log_net_event(crate::telemetry::net_log::NetEventType::CacheHit, req.url.as_str(), "Sparse Range Hit");
                    
                    let mut headers = req.headers.clone();
                    headers.insert(
                        http::header::CONTENT_RANGE, 
                        http::HeaderValue::from_str(&format!("bytes {}-{}/*", requested_start, requested_end)).unwrap()
                    );

                    return Ok((None, Some(Response {
                        url: req.url.clone(),
                        status: StatusCode::PARTIAL_CONTENT,
                        headers,
                        body: ResponseBody::Full(bytes),
                        mime_type: "application/octet-stream".into(),
                        charset: None,
                        content_encoding: None,
                        retry_after: None,
                        content_range: Some(crate::http::range::ContentRange { start: requested_start, end: requested_end, total: None }),
                        from_cache: true,
                        timing: ResponseTiming::default(),
                    })));
                }
            } else if let Some(entry) = self.cache.get(nik, &req.url).await {
                if entry.matches_request_headers(&req.headers) {
                    if entry.is_fresh(now) {
                        if let Some(ref digests) = req.integrity {
                            crate::security::sri::verify_integrity(entry.body.as_ref(), digests)?;
                        }
                        self.metrics.cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        self.metrics.bytes_served_from_cache.fetch_add(
                            entry.body.len() as u64,
                            std::sync::atomic::Ordering::Relaxed,
                        );
                        crate::telemetry::net_log::log_net_event(crate::telemetry::net_log::NetEventType::CacheHit, req.url.as_str(), "Fresh Hit");
                        
                        return Ok((None, Some(Response {
                            url: req.url.clone(),
                            status: entry.status,
                            headers: entry.headers.clone(),
                            body: ResponseBody::Full(entry.body.clone()),
                            mime_type: entry.headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).map(|s| s.split(';').next().unwrap_or("").trim().into()).unwrap_or_else(|| "application/octet-stream".into()),
                            charset: entry.headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).and_then(extract_charset_from_content_type),
                            content_encoding: None,
                            retry_after: None,
                            content_range: None,
                            from_cache: true,
                            timing: ResponseTiming::default(),
                        })));
                    } else if entry.is_stale_revalidatable(now) {
                        if let Some(ref digests) = req.integrity {
                            crate::security::sri::verify_integrity(entry.body.as_ref(), digests)?;
                        }
                        self.metrics.cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        self.metrics.cache_revalidations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        self.metrics.bytes_served_from_cache.fetch_add(
                            entry.body.len() as u64,
                            std::sync::atomic::Ordering::Relaxed,
                        );
                        crate::telemetry::net_log::log_net_event(crate::telemetry::net_log::NetEventType::CacheHit, req.url.as_str(), "Stale-While-Revalidate Hit");
                        
                        let bg_fetcher = self.clone();
                        let mut bg_req = req.clone();
                        let cond_headers = entry.conditional_headers();
                        for (k, v) in cond_headers {
                            if let Some(name) = k {
                                bg_req.headers.insert(name, v);
                            }
                        }
                        let bg_nik = nik.cloned();
                        let bg_entry = entry.clone();
                        tokio::spawn(async move {
                            let _ = bg_fetcher.revalidate_background(bg_req, bg_nik, bg_entry).await;
                        });

                        return Ok((None, Some(Response {
                            url: req.url.clone(),
                            status: entry.status,
                            headers: entry.headers.clone(),
                            body: ResponseBody::Full(entry.body.clone()),
                            mime_type: entry.headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).map(|s| s.split(';').next().unwrap_or("").trim().into()).unwrap_or_else(|| "application/octet-stream".into()),
                            charset: entry.headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).and_then(extract_charset_from_content_type),
                            content_encoding: None,
                            retry_after: None,
                            content_range: None,
                            from_cache: true,
                            timing: ResponseTiming::default(),
                        })));
                    } else {
                        self.metrics.cache_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        self.metrics.cache_revalidations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        crate::telemetry::net_log::log_net_event(crate::telemetry::net_log::NetEventType::CacheMiss, req.url.as_str(), "Stale - Needs Revalidation");

                        let cond_headers = entry.conditional_headers();
                        for (k, v) in cond_headers {
                            if let Some(name) = k {
                                req.headers.insert(name, v);
                            }
                        }
                        cached_entry = Some(entry);
                    }
                } else {
                    self.metrics.cache_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    crate::telemetry::net_log::log_net_event(crate::telemetry::net_log::NetEventType::CacheMiss, req.url.as_str(), "Vary Mismatch");
                }
            } else {
                self.metrics.cache_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                crate::telemetry::net_log::log_net_event(crate::telemetry::net_log::NetEventType::CacheMiss, req.url.as_str(), "Not found in cache");
            }
        }
        Ok((cached_entry, None))
    }

    fn apply_referrer_policy(&self, req: &mut Request) {
        if let Some(ref referrer_url) = req.referrer {
            let current_origin = Origin::parse(referrer_url.as_str()).unwrap_or_else(|_| Origin::new_opaque());
            if let Some(computed_ref) = compute_referrer(&current_origin, referrer_url.as_str(), req.url.as_str(), req.referrer_policy) {
                if let Ok(val) = HeaderValue::from_str(&computed_ref) {
                    req.headers.insert(REFERER, val);
                }
            }
        }
    }

    fn apply_compression_dictionary_hints(&self, req: &mut Request, nik: Option<&crate::cache::NetworkIsolationKey>, now: SystemTime) {
        if let Some(dict) = self.dictionary_manager.find_dictionary_for_url(&req.url, nik, now) {
            if let Ok(hdr_val) = HeaderValue::from_str(&dict.available_dictionary_header_value()) {
                req.headers.insert("available-dictionary", hdr_val);
            }
            let mut ae = req.headers.get(http::header::ACCEPT_ENCODING)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("gzip, deflate, br, zstd")
                .to_string();
            if !ae.contains("dcb") {
                ae.push_str(", dcb, dcz");
                if let Ok(new_ae) = HeaderValue::from_str(&ae) {
                    req.headers.insert(http::header::ACCEPT_ENCODING, new_ae);
                }
            }
        }
    }

    async fn execute_network_transport(
        &self,
        mut current_req: Request,
        nik: Option<&crate::cache::NetworkIsolationKey>,
        mut current_cached_entry: Option<CacheEntry>,
        now: SystemTime,
    ) -> NetResult<Response> {
        let max_redirects = match current_req.redirect_policy {
            crate::http::request::RedirectPolicy::Follow(max) => max,
            crate::http::request::RedirectPolicy::Manual => 0,
            crate::http::request::RedirectPolicy::Error => 0,
        };

        let mut visited_urls = HashSet::new();
        visited_urls.insert(current_req.url.to_string());

        let top_level_site = nik.and_then(|k| match &k.top_frame_origin {
            Origin::Tuple { host, .. } => Some(host.as_str().to_string()),
            Origin::Opaque(_) => None,
        });

        loop {
            let request_time = SystemTime::now();
            
            if let Some(alt_svc) = self.alt_svc_registry.get(current_req.url.host_str().unwrap_or(""), current_req.url.port().unwrap_or(443)) {
                if alt_svc.protocol_id.starts_with("h3") {
                    current_req.force_h3 = true;
                }
            }
            
            let host_smol = current_req.url.host_str().unwrap_or("").into();
            let _permit = self.scheduler.acquire(current_req.id, current_req.priority, host_smol).await;
            
            let is_idempotent = current_req.method == Method::GET || current_req.method == Method::HEAD || current_req.method == Method::OPTIONS;
            let max_attempts = if is_idempotent { 3 } else { 1 };
            let mut attempt = 0;

            let mut network_response = loop {
                attempt += 1;
                let network_result = self.transport.execute(&current_req).await;
                match network_result {
                    Ok(resp) => {
                        let is_transient_status = resp.status == StatusCode::SERVICE_UNAVAILABLE 
                            || resp.status == StatusCode::TOO_MANY_REQUESTS 
                            || resp.status == StatusCode::BAD_GATEWAY;

                        if attempt < max_attempts && is_transient_status {
                            self.metrics.retried_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            crate::telemetry::net_log::log_net_event(
                                crate::telemetry::net_log::NetEventType::Retry,
                                current_req.url.as_str(),
                                &format!("Status {}, disparando retentativa {}/{}", resp.status, attempt, max_attempts),
                            );

                            let delay = if let Some(crate::engine::contention::RetryAfter::Seconds(s)) = resp.retry_after {
                                std::cmp::min(s, Duration::from_secs(2))
                            } else {
                                Duration::from_millis(50 * (1 << (attempt - 1)))
                            };

                            tokio::select! {
                                _ = current_req.cancellation_token.cancelled() => return Err(NetError::Cancelled),
                                _ = tokio::time::sleep(delay) => {},
                            }
                            continue;
                        }

                        if resp.status.is_server_error() {
                            if let Some(ref cached) = current_cached_entry {
                                if cached.is_stale_if_error(now) {
                                    crate::telemetry::net_log::log_net_event(
                                        crate::telemetry::net_log::NetEventType::Warning,
                                        current_req.url.as_str(),
                                        "Servidor retornou 5xx; servindo cache via stale-if-error (RFC 5861)",
                                    );
                                    let mut stale_resp = Response {
                                        url: current_req.url.clone(),
                                        status: cached.status,
                                        headers: cached.headers.clone(),
                                        body: ResponseBody::Full(cached.body.clone()),
                                        mime_type: cached.headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).map(|s| s.split(';').next().unwrap_or("").trim().into()).unwrap_or_else(|| "application/octet-stream".into()),
                                        charset: cached.headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).and_then(extract_charset_from_content_type),
                                        content_encoding: None,
                                        retry_after: None,
                                        content_range: None,
                                        from_cache: true,
                                        timing: ResponseTiming::default(),
                                    };
                                    stale_resp.headers.insert(
                                        http::header::WARNING,
                                        HeaderValue::from_static("110 - \"Response is Stale\""),
                                    );
                                    return Ok(stale_resp);
                                }
                            }
                        }
                        break resp;
                    }
                    Err(err) => {
                        let is_transient_err = matches!(err, NetError::ConnectionFailed(_, _) | NetError::Timeout);
                        if attempt < max_attempts && is_transient_err {
                            self.metrics.retried_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            crate::telemetry::net_log::log_net_event(
                                crate::telemetry::net_log::NetEventType::Retry,
                                current_req.url.as_str(),
                                &format!("Erro transitório ({}), disparando retentativa {}/{}", err, attempt, max_attempts),
                            );
                            let delay = Duration::from_millis(50 * (1 << (attempt - 1)));
                            tokio::select! {
                                _ = current_req.cancellation_token.cancelled() => return Err(NetError::Cancelled),
                                _ = tokio::time::sleep(delay) => {},
                            }
                            continue;
                        }

                        if let Some(ref cached) = current_cached_entry {
                            if cached.is_stale_if_error(now) {
                                crate::telemetry::net_log::log_net_event(
                                    crate::telemetry::net_log::NetEventType::Warning,
                                    current_req.url.as_str(),
                                    &format!("Falha de rede ({}); servindo cache via stale-if-error (RFC 5861)", err),
                                );
                                let mut stale_resp = Response {
                                    url: current_req.url.clone(),
                                    status: cached.status,
                                    headers: cached.headers.clone(),
                                    body: ResponseBody::Full(cached.body.clone()),
                                    mime_type: cached.headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).map(|s| s.split(';').next().unwrap_or("").trim().into()).unwrap_or_else(|| "application/octet-stream".into()),
                                    charset: cached.headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).and_then(extract_charset_from_content_type),
                                    content_encoding: None,
                                    retry_after: None,
                                    content_range: None,
                                    from_cache: true,
                                    timing: ResponseTiming::default(),
                                };
                                stale_resp.headers.insert(
                                    http::header::WARNING,
                                    HeaderValue::from_static("110 - \"Response is Stale\""),
                                );
                                return Ok(stale_resp);
                            }
                        }
                        return Err(err);
                    }
                }
            };
            let response_time = SystemTime::now();

            self.cookie_jar.process_response_headers(
                &current_req.url,
                &network_response.headers,
                top_level_site.as_deref(),
                response_time,
            );

            if current_req.url.scheme() == "https" {
                if let Some(hsts_val) = network_response.headers.get(STRICT_TRANSPORT_SECURITY) {
                    if let Some(host) = current_req.url.host_str() {
                        self.hsts_store.update_from_header(host, hsts_val, response_time);
                    }
                }
            }

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

            if let Some(alt_svc_raw) = network_response.headers.get("alt-svc").and_then(|v| v.to_str().ok()) {
                let records = parse_alt_svc(alt_svc_raw, response_time);
                if let Some(host) = current_req.url.host_str() {
                    self.alt_svc_registry.insert(nik, host, records);
                }
            }

            if network_response.status == StatusCode::NOT_MODIFIED {
                if let Some(mut stale_entry) = current_cached_entry {
                    let updated = self.cache.update_304(
                        nik,
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
                        mime_type: final_entry.headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).map(|s| s.split(';').next().unwrap_or("").trim().into()).unwrap_or_else(|| "application/octet-stream".into()),
                        charset: final_entry.headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).and_then(extract_charset_from_content_type),
                        content_encoding: None,
                        retry_after: None,
                        content_range: None,
                        from_cache: true,
                        timing: network_response.timing,
                    });
                }
            }

            if max_redirects > 0 {
                let redirect_action = handle_redirect(
                    &current_req,
                    network_response.status,
                    &network_response.headers,
                    &mut visited_urls,
                    max_redirects,
                )?;

                if let RedirectAction::Follow(follow) = redirect_action {
                    current_req.url = follow.new_url;
                    current_req.method = follow.new_method;
                    current_req.body = follow.new_body;
                    current_req.headers = follow.new_headers;
                    current_cached_entry = self.cache.get(nik, &current_req.url).await;
                    continue;
                }
            }

            crate::security::cors::validate_cors_response(&current_req, &network_response)?;

            // W3C CORP (Cross-Origin Resource Policy)
            let corp_policy = crate::security::isolation::extract_corp_policy(&network_response.headers);
            crate::security::isolation::validate_corp(
                current_req.initiator.as_ref(),
                &current_req.url,
                corp_policy,
            )?;

            // WHATWG nosniff & MIME Sniffing
            let nosniff = crate::http::sniffing::is_nosniff_header_present(&network_response.headers);
            if nosniff {
                if !crate::http::sniffing::validate_nosniff_content_type(current_req.destination, &network_response.mime_type) {
                    return Err(NetError::SecurityViolation(format!(
                        "MIME type '{}' incompatível com destino '{:?}' sob X-Content-Type-Options: nosniff",
                        network_response.mime_type, current_req.destination
                    )));
                }
            } else if crate::http::sniffing::is_unknown_mime_type(Some(&network_response.mime_type)) {
                let sniffed = crate::http::sniffing::sniff_mime_type(
                    Some(&network_response.mime_type),
                    network_response.body.as_bytes(),
                    false,
                );
                network_response.mime_type = sniffed;
            }

            if let Some(ref digests) = current_req.integrity {
                crate::security::sri::verify_integrity(network_response.body.as_bytes(), digests)?;
            }

            self.metrics.bytes_transferred.fetch_add(
                network_response.body.as_bytes().len() as u64,
                std::sync::atomic::Ordering::Relaxed,
            );

            // Registro de novo dicionário via Use-As-Dictionary (RFC 9290)
            if network_response.status == StatusCode::OK {
                if let Some(uad_val) = network_response.headers.get("use-as-dictionary").and_then(|v| v.to_str().ok()) {
                    if let Some(directive) = crate::http::dictionary::UseAsDictionaryDirective::parse(uad_val) {
                        if let Ok(origin) = Origin::parse(current_req.url.as_str()) {
                            let body_bytes = match &network_response.body {
                                ResponseBody::Full(b) => b.clone(),
                                _ => bytes::Bytes::new(),
                            };
                            self.dictionary_manager.register_dictionary(
                                nik,
                                origin,
                                body_bytes,
                                directive,
                                response_time,
                            );
                        }
                    }
                }
            }

            // Descompressão assistida por dicionário para dcb (Brotli) ou dcz (Zstandard)
            if let Some(enc_val) = network_response.headers.get(http::header::CONTENT_ENCODING).and_then(|v| v.to_str().ok()) {
                let enc_lower = enc_val.trim().to_ascii_lowercase();
                if enc_lower == "dcb" || enc_lower == "dcz" {
                    if let Some(dict) = self.dictionary_manager.find_dictionary_for_url(&current_req.url, nik, response_time) {
                        if let ResponseBody::Full(ref comp_bytes) = network_response.body {
                            if let Ok(decomp) = crate::http::dictionary::decompress_with_dictionary(&enc_lower, comp_bytes.as_ref(), dict.content.as_ref()) {
                                network_response.body = ResponseBody::Full(decomp);
                            }
                        }
                    }
                }
            }

            if network_response.status == StatusCode::PARTIAL_CONTENT {
                if let Some(cr_val) = network_response.headers.get(http::header::CONTENT_RANGE) {
                    if let Some(cr) = crate::http::range::ContentRange::parse(cr_val) {
                        if CacheEntry::is_cacheable(&current_req.method, network_response.status, &network_response.headers) {
                            if let ResponseBody::Full(ref bytes) = network_response.body {
                                self.cache.put_range(
                                    nik,
                                    current_req.url.clone(),
                                    cr.start,
                                    cr.end,
                                    cr.total,
                                    bytes.clone()
                                );
                            }
                        }
                    }
                }
            } else if CacheEntry::is_cacheable(&current_req.method, network_response.status, &network_response.headers) {
                if let ResponseBody::Full(ref bytes) = network_response.body {
                    let entry = CacheEntry::new(
                        current_req.url.clone(),
                        network_response.status,
                        network_response.headers.clone(),
                        bytes.clone(),
                        request_time,
                        response_time,
                    ).with_request_headers(current_req.headers.clone());
                    self.cache.put(nik, current_req.url.clone(), entry);
                }
            }

            return Ok(network_response);
        }
    }
    /// Executa revalidação de cache assíncrona em background para RFC 5861 (stale-while-revalidate).
    async fn revalidate_background(
        &self,
        req: Request,
        nik: Option<crate::cache::NetworkIsolationKey>,
        _stale_entry: CacheEntry,
    ) -> NetResult<()> {
        let resp_time = SystemTime::now();
        if let Ok(network_response) = self.transport.execute(&req).await {
            if network_response.status == StatusCode::NOT_MODIFIED {
                self.cache.update_304(nik.as_ref(), &req.url, &network_response.headers, resp_time);
            } else if CacheEntry::is_cacheable(&req.method, network_response.status, &network_response.headers) {
                if let ResponseBody::Full(ref bytes) = network_response.body {
                    let entry = CacheEntry::new(
                        req.url.clone(),
                        network_response.status,
                        network_response.headers.clone(),
                        bytes.clone(),
                        resp_time,
                        resp_time,
                    ).with_request_headers(req.headers);
                    self.cache.put(nik.as_ref(), req.url, entry);
                }
            }
        }
        Ok(())
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

    /// Reprioritiza requisições com base na visibilidade do Viewport.
    /// Chamado pelo DOM/Layout engine para imagens que entraram na tela, passando a elas `PriorityLevel::High`.
    pub fn boost_viewport_visibility(&self, visible_request_ids: &[RequestId]) {
        for &req_id in visible_request_ids {
            // Em M4 contornamos a falta de reprioritização HTTP/2 ativa no hyper_util
            // reprioritizando apenas na nossa fila mestre local do Scheduler.
            if self.scheduler.reprioritize(req_id, PriorityLevel::High) {
                crate::telemetry::net_log::log_net_event(
                    crate::telemetry::net_log::NetEventType::Queue,
                    "viewport",
                    &format!("Boosted priority for request {:?} to High based on Viewport visibility", req_id),
                );
            }
        }
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

    struct MockServiceWorker;
    impl ServiceWorkerHook for MockServiceWorker {
        fn on_fetch(
            &self,
            req: &Request,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NetResult<Option<Response>>> + Send + '_>> {
            let url = req.url.clone();
            Box::pin(async move {
                if url.as_str() == "https://sw-intercepted.local/offline" {
                    let mut headers = http::HeaderMap::new();
                    headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/html"));
                    Ok(Some(Response {
                        url,
                        status: StatusCode::OK,
                        headers,
                        body: ResponseBody::Full(bytes::Bytes::from_static(b"<h1>From ServiceWorker</h1>")),
                        mime_type: "text/html".into(),
                        charset: Some("utf-8".into()),
                        content_encoding: None,
                        retry_after: None,
                        content_range: None,
                        from_cache: false,
                        timing: ResponseTiming::default(),
                    }))
                } else {
                    Ok(None)
                }
            })
        }
    }

    #[tokio::test]
    async fn test_fetcher_service_worker_interception() {
        let fetcher = ResourceFetcher::new().unwrap();
        fetcher.set_service_worker_hook(Arc::new(MockServiceWorker));

        // URL interceptada pelo Service Worker
        let req = Request::get("https://sw-intercepted.local/offline").unwrap().build();
        let resp = fetcher.fetch(req).await.unwrap();

        assert_eq!(resp.status, StatusCode::OK);
        assert_eq!(resp.text().unwrap(), "<h1>From ServiceWorker</h1>");
        assert_eq!(resp.mime_type.as_str(), "text/html");

        // Limpa o hook e verifica que não intercepta mais
        fetcher.clear_service_worker_hook();
        let req2 = Request::get("data:text/plain,direct").unwrap().build();
        let resp2 = fetcher.fetch(req2).await.unwrap();
        assert_eq!(resp2.text().unwrap(), "direct");
    }
}
