//! # Primitivas de Rede e Protocolos Web
//!
//! Estruturas fundamentais para negociação de conteúdo, cabeçalhos, URIs e tipos de mídia MIME da web.

pub mod data_url;
pub mod mime;
pub mod percent;
pub mod utils;

pub use data_url::{parse_data_url, DataUrlRecord};
pub use mime::{sniff_mime_type, MimeType};
pub use percent::{percent_encode, percent_encode_byte, PercentEncodeSet};
pub use utils::{is_safe_url_scheme, parse_data_uri, percent_decode};
