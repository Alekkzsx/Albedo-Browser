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
    let current_scheme = parsed_current.scheme();
    if current_scheme != "http" && current_scheme != "https" {
        return None;
    }

    let target_scheme = parsed_target.scheme();
    let is_downgrade = current_scheme == "https" && target_scheme != "https";

    // Extrai origem do alvo diretamente da URL já parseada (zero re-parsing)
    let target_origin_scheme = match target_scheme {
        "http" => crate::security::origin::Scheme::Http,
        "https" => crate::security::origin::Scheme::Https,
        "file" => crate::security::origin::Scheme::File,
        other => crate::security::origin::Scheme::Custom(smol_str::SmolStr::new(other)),
    };
    let target_host = match parsed_target.host() {
        Some(url::Host::Domain(d)) => crate::security::origin::Host::Domain(smol_str::SmolStr::new(d)),
        Some(url::Host::Ipv4(ip)) => crate::security::origin::Host::Ip(std::net::IpAddr::V4(ip)),
        Some(url::Host::Ipv6(ip)) => crate::security::origin::Host::Ip(std::net::IpAddr::V6(ip)),
        None => crate::security::origin::Host::Opaque,
    };
    let target_origin = Origin::tuple(target_origin_scheme, target_host, parsed_target.port());
    let is_same_origin = current_origin.same_origin(&target_origin);

    // Helpers lazy para evitar alocações quando descartado
    let get_sanitized_url = || {
        let mut sanitized = parsed_current.clone();
        sanitized.set_fragment(None);
        let _ = sanitized.set_username("");
        let _ = sanitized.set_password(None);
        sanitized.to_string()
    };

    let get_origin_string = || current_origin.ascii_serialization();

    match policy {
        ReferrerPolicy::NoReferrer => None,
        ReferrerPolicy::NoReferrerWhenDowngrade => {
            if is_downgrade {
                None
            } else {
                Some(get_sanitized_url())
            }
        }
        ReferrerPolicy::SameOrigin => {
            if is_same_origin {
                Some(get_sanitized_url())
            } else {
                None
            }
        }
        ReferrerPolicy::Origin => Some(get_origin_string()),
        ReferrerPolicy::StrictOrigin => {
            if is_downgrade {
                None
            } else {
                Some(get_origin_string())
            }
        }
        ReferrerPolicy::OriginWhenCrossOrigin => {
            if is_same_origin {
                Some(get_sanitized_url())
            } else {
                Some(get_origin_string())
            }
        }
        ReferrerPolicy::StrictOriginWhenCrossOrigin => {
            if is_same_origin {
                Some(get_sanitized_url())
            } else if is_downgrade {
                None
            } else {
                Some(get_origin_string())
            }
        }
        ReferrerPolicy::UnsafeUrl => Some(get_sanitized_url()),
    }
}

