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
//! - **Fila de Prioridades:** Escalonamento de prioridades semânticas (VeryHigh para HTML, High para CSS/JS/Fontes, Medium para Imagens).
//! - **Redirecionamento Seguro:** Detecção de loops, limite de saltos e descarte de credenciais em travessias cross-origin.
//! - **Decodificação & Encodings:** Detecção de BOM e charset via WHATWG Encoding standard.

pub mod cache;
pub mod encoding;
pub mod error;
pub mod fetcher;
pub mod priority;
pub mod redirect;
pub mod request;
pub mod response;
pub mod transport;

pub use cache::{CacheEntry, CacheStats, HttpCache, NetworkIsolationKey};
pub use encoding::{decode_to_string, extract_charset_from_content_type, sniff_bom, DetectedEncoding};
pub use error::{NetError, NetResult};
pub use fetcher::ResourceFetcher;
pub use priority::{PrioritizedItem, PriorityLevel};
pub use redirect::{handle_redirect, RedirectAction};
pub use request::{
    CredentialsMode, HeaderMap, HeaderName, HeaderValue, Method, RedirectPolicy, Request,
    RequestBuilder, RequestDestination, RequestMode, TryIntoUrl,
};
pub use response::{Response, ResponseBody, ResponseTiming, StatusCode};
pub use transport::TransportClient;
pub use url::Url;

/// Cria uma instância padrão do `ResourceFetcher` para uso imediato pelo motor.
pub fn create_default_fetcher() -> NetResult<ResourceFetcher> {
    ResourceFetcher::new()
}
