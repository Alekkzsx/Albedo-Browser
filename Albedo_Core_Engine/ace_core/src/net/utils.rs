//! # Utilitários de Rede, Decodificação e URIs
//!
//! Funções auxiliares para decodificação de `data:` URIs (RFC 2397), percent-decoding e validação de esquemas de rede.

/// Valida se um esquema de protocolo é seguro e padrão para navegação na web.
#[inline]
pub fn is_safe_url_scheme(scheme: &str) -> bool {
    matches!(
        scheme.to_ascii_lowercase().as_str(),
        "http" | "https" | "data" | "blob" | "about" | "file"
    )
}

