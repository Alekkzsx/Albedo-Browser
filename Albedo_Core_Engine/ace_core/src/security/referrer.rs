//! # Motor de Resolução de Referrer Policy (W3C Referrer Policy)
//!
//! Implementação canônica da política de controle de envio do cabeçalho HTTP `Referer`
//! para proteção de privacidade contra vazamento de URLs entre origens distintas.

use crate::security::origin::Origin;

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

/// Computa o valor canônico do cabeçalho `Referer` conforme a especificação W3C Referrer Policy (RFC 9110 §10.1.4).
pub fn compute_referrer(
    current_origin: &Origin,
    current_url: &str,
    target_url: &str,
    policy: ReferrerPolicy,
) -> Option<String> {
    if policy == ReferrerPolicy::NoReferrer {
        return None;
    }

    let parsed_current = url::Url::parse(current_url).ok()?;
    let parsed_target = url::Url::parse(target_url).ok()?;

    // Apenas esquemas HTTP/HTTPS são elegíveis para envio de Referer
    if parsed_current.scheme() != "http" && parsed_current.scheme() != "https" {
        return None;
    }

    let is_downgrade = parsed_current.scheme() == "https" && parsed_target.scheme() != "https";
    let target_origin = Origin::parse(target_url).ok()?;
    let is_same_origin = current_origin.same_origin(&target_origin);

    // Sanitiza: remove fragmentos (#...) e dados de autenticação sensíveis (userinfo)
    let mut sanitized_url = parsed_current.clone();
    sanitized_url.set_fragment(None);
    let _ = sanitized_url.set_username("");
    let _ = sanitized_url.set_password(None);
    let sanitized_current_url = sanitized_url.to_string();

    let origin_string = current_origin.ascii_serialization();

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

