//! # Primitivas de Rede e Protocolos Web
//!
//! Estruturas fundamentais para negociação de conteúdo, cabeçalhos, URIs e tipos de mídia MIME da web.

pub mod mime;
pub mod utils;

pub use mime::{sniff_mime_type, MimeType};
pub use utils::{is_safe_url_scheme, parse_data_uri, percent_decode};
