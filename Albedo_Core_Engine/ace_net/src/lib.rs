//! # Albedo Core Engine — ace_net
//!
//! O motor de conectividade, transporte seguro TLS 1.3, HTTP/1.1 e HTTP/2,
//! fila de priorização WHATWG e cache normativo RFC 9111 do **Albedo Browser**.
//!
//! ## Principais Recursos
//! - **ResourceFetcher:** Orquestrador unificado de busca e pré-carregamento de recursos.
//! - **Transporte HTTP Concorrente:** ALPN automático (HTTP/2 e HTTP/1.1) sobre `hyper` e `rustls` (WebPKI).
//! - **HTTP Cache RFC 9111:** Cache em memória com despejo LRU, revalidação condicional `304 Not Modified` e controle estrito de `max-age`/`no-store`.
//! - **Network State Partitioning:** Isolamento de chaves de cache por `NetworkIsolationKey` (Top-Frame Origin + Frame Origin) contra rastreamento cross-site.
//! - **Cookie Jar Normativo (RFC 6265bis + CHIPS):** Particionamento de cookies de terceiros por TopLevelSite (`Partitioned`).
//! - **HSTS & Preload List (RFC 6797):** Auto-upgrade de conexões `http://` para `https://` em memória prevenindo SSL stripping.
//! - **W3C Fetch Metadata:** Injeção automática das diretivas de segurança `Sec-Fetch-Site`, `Sec-Fetch-Mode`, `Sec-Fetch-Dest` e `Sec-Fetch-User`.
//! - **W3C Clear-Site-Data (RFC 8879):** Purga automática de cache e cookies em logout ou redefinição de segurança.
//! - **Range Requests & 206 Partial Content:** Suporte normativo a faixas de bytes para streaming de mídia e retomada de downloads.
//! - **Client Hints (RFC 8942):** Metadados de plataforma e motor (`Sec-CH-UA`).
//! - **Priorização Extensível RFC 9218:** Injeção automática de `Priority: u=..., i` e escalonamento semântico WHATWG.
//! - **Descompressão Transparente:** Suporte nativo a `gzip`, `deflate` e `br` (Brotli).
//! - **Cancelamento Granular:** Abort de requisições ativas via `RequestId` e `CancellationToken`.
//! - **Tratamento de Contenção:** Suporte a `Retry-After` (RFC 9110) para status 429 e 503.
//! - **Alt-Svc Registry:** Registro de serviços alternativos (RFC 7838) para preparação HTTP/3.
//! - **Resource Hints:** Suporte a `dns-prefetch`, `preconnect` e lookahead com `PreloadScanner`.

pub mod alt_svc;
pub mod cache;
pub mod cancel;
pub mod clear_site_data;
pub mod client_hints;
pub mod compression;
pub mod contention;
pub mod cookie;
pub mod encoding;
pub mod cors;
pub mod error;
pub mod fetch_metadata;
pub mod fetcher;
pub mod hints;
pub mod hsts;
pub mod metrics;
pub mod net_log;
pub mod pipeline;
pub mod priority;
pub mod range;
pub mod redirect;
pub mod request;
pub mod response;
pub mod scheduler;
pub mod service_worker_hook;
pub mod transport;
pub mod websocket;

pub use ace_core::id::RequestId;
pub use alt_svc::{parse_alt_svc, AltSvcRecord, AltSvcRegistry};
pub use cache::{CacheEntry, CacheStats, HttpCache, NetworkIsolationKey};
pub use cancel::CancellationRegistry;
pub use clear_site_data::ClearSiteDataAction;
pub use client_hints::{DEFAULT_SEC_CH_UA, DEFAULT_SEC_CH_UA_PLATFORM};
pub use compression::{decompress_payload, ContentEncoding};
pub use contention::RetryAfter;
pub use cookie::{is_public_suffix, is_valid_cookie_domain, Cookie, CookieJar, SameSite, MAX_COOKIES_PER_DOMAIN};
pub use encoding::{decode_to_string, extract_charset_from_content_type, sniff_bom, DetectedEncoding};
pub use error::{NetError, NetResult};
pub use fetch_metadata::{SecFetchDest, SecFetchMode, SecFetchSite};
pub use fetcher::ResourceFetcher;
pub use hints::ResourceHint;
pub use hsts::{HstsPolicy, HstsStore};
pub use metrics::FetcherMetrics;
pub use net_log::{log_net_error, log_net_event, NetEventType};
pub use priority::{PrioritizedItem, PriorityLevel};
pub use range::{ByteRangeSpec, ContentRange};
pub use redirect::{handle_redirect, RedirectAction};
pub use request::{
    CredentialsMode, HeaderMap, HeaderName, HeaderValue, Method, RedirectPolicy, Request,
    RequestBuilder, RequestDestination, RequestMode, TryIntoUrl,
};
pub use response::{BoxByteStream, Response, ResponseBody, ResponseTiming, StatusCode};
pub use scheduler::{ResourceScheduler, SchedulerConfig, SchedulerPermit};
pub use service_worker_hook::ServiceWorkerHook;
pub use tokio_util::sync::CancellationToken;
pub use transport::{DohHappyEyeballsResolver, TransportClient};
pub use url::Url;
pub use websocket::{WebSocketMessage, WebSocketSession, WebSocketUpgrade};

/// Cria uma instância padrão do `ResourceFetcher` para uso imediato pelo motor.
pub fn create_default_fetcher() -> NetResult<ResourceFetcher> {
    ResourceFetcher::new()
}
