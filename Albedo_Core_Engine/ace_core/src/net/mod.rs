//! # Primitivas de Rede e Protocolos Web
//!
//! Estruturas fundamentais para negociação de conteúdo, cabeçalhos e tipos de mídia MIME da web.

pub mod mime;

pub use mime::{sniff_mime_type, MimeType};
