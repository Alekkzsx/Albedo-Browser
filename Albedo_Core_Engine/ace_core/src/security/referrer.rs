//! # Motor de Resolução de Referrer Policy (W3C Referrer Policy)
//!
//! Implementação canônica da política de controle de envio do cabeçalho HTTP `Referer`
//! para proteção de privacidade contra vazamento de URLs entre origens distintas.

use crate::security::origin::{Origin, Scheme};

/// Políticas de Referrer padronizadas pelo W3C.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ReferrerPolicy {
    /// Nunca envia o cabeçalho Referer.
    NoReferrer,
    /// Não envia em caso de downgrade HTTPS -> HTTP (padrão legado).
    NoReferrerWhenDowngrade,
    /// Envia apenas a origem (esquema + host + porta), omitindo o caminho e a query.
    Origin,
    /// Envia a URL completa para a mesma origem, e apenas a origem para requisições cross-origin.
    OriginWhenCrossOrigin,
    /// Envia a URL completa para a mesma origem; descarta completamente para cross-origin.
    SameOrigin,
    /// Envia a origem, exceto em caso de downgrade HTTPS -> HTTP onde é omitido.
    StrictOrigin,
    /// Envia a URL completa para a mesma origem, apenas a origem para cross-origin seguro,
    /// e descarta em caso de downgrade HTTPS -> HTTP (padrão moderno padrão dos navegadores).
    #[default]
    StrictOriginWhenCrossOrigin,
    /// Envia sempre a URL completa (inseguro).
    UnsafeUrl,
}

impl ReferrerPolicy {
    /// Converte uma palavra-chave CSS/HTML/HTTP no enum `ReferrerPolicy`.
    pub fn from_str_policy(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "no-referrer" => Some(Self::NoReferrer),
            "no-referrer-when-downgrade" => Some(Self::NoReferrerWhenDowngrade),
            "origin" => Some(Self::Origin),
            "origin-when-cross-origin" => Some(Self::OriginWhenCrossOrigin),
            "same-origin" => Some(Self::SameOrigin),
            "strict-origin" => Some(Self::StrictOrigin),
            "strict-origin-when-cross-origin" => Some(Self::StrictOriginWhenCrossOrigin),
            "unsafe-url" => Some(Self::UnsafeUrl),
            _ => None,
        }
    }

    /// Retorna o nome canônico em string da política.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoReferrer => "no-referrer",
            Self::NoReferrerWhenDowngrade => "no-referrer-when-downgrade",
            Self::Origin => "origin",
            Self::OriginWhenCrossOrigin => "origin-when-cross-origin",
            Self::SameOrigin => "same-origin",
            Self::StrictOrigin => "strict-origin",
            Self::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin",
            Self::UnsafeUrl => "unsafe-url",
        }
    }
}

/// Computa o valor canônico do cabeçalho `Referer` conforme a especificação W3C Referrer Policy.
pub fn compute_referrer(
    current_origin: &Origin,
    current_url: &str,
    target_url: &str,
    policy: ReferrerPolicy,
) -> Option<String> {
    if policy == ReferrerPolicy::NoReferrer {
        return None;
    }

    // Apenas esquemas HTTP/HTTPS são elegíveis para envio de Referer
    if !current_url.starts_with("http://") && !current_url.starts_with("https://") {
        return None;
    }

    let is_current_https = current_url.starts_with("https://");
    let is_target_https = target_url.starts_with("https://");
    let is_downgrade = is_current_https && !is_target_https;

    // Remove fragmentos (#...) e dados de autenticação sensíveis da URL atual
    let sanitized_current_url = strip_url_fragment(current_url);
    let origin_string = current_origin.ascii_serialization();

    let is_same_origin = target_url.starts_with(&origin_string);

    match policy {
        ReferrerPolicy::NoReferrer => None,
        ReferrerPolicy::NoReferrerWhenDowngrade => {
            if is_downgrade {
                None
            } else {
                Some(sanitized_current_url)
            }
        }
        ReferrerPolicy::SameOrigin => {
            if is_same_origin {
                Some(sanitized_current_url)
            } else {
                None
            }
        }
        ReferrerPolicy::Origin => Some(origin_string),
        ReferrerPolicy::StrictOrigin => {
            if is_downgrade {
                None
            } else {
                Some(origin_string)
            }
        }
        ReferrerPolicy::OriginWhenCrossOrigin => {
            if is_same_origin {
                Some(sanitized_current_url)
            } else {
                Some(origin_string)
            }
        }
        ReferrerPolicy::StrictOriginWhenCrossOrigin => {
            if is_same_origin {
                Some(sanitized_current_url)
            } else if is_downgrade {
                None
            } else {
                Some(origin_string)
            }
        }
        ReferrerPolicy::UnsafeUrl => Some(sanitized_current_url),
    }
}

/// Remove fragmentos (`#...`) da URL para evitar vazamento de estado de âncora.
fn strip_url_fragment(url: &str) -> String {
    if let Some(idx) = url.find('#') {
        url[..idx].to_string()
    } else {
        url.to_string()
    }
}
