//! # Modelo Canônico de Origem Web e Same-Origin Policy (RFC 6454)
//!
//! Representação de origens de segurança para isolamento de cookies, storage, CORS e scripts.

use crate::error::AceError;
use smol_str::SmolStr;
use std::fmt;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};

static OPAQUE_ORIGIN_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Esquema de protocolo de uma URL.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scheme {
    Http,
    Https,
    File,
    Custom(SmolStr),
}

impl Scheme {
    /// Retorna a porta TCP padrão para este esquema (ex: 80 para HTTP, 443 para HTTPS).
    pub fn default_port(&self) -> Option<u16> {
        match self {
            Self::Http => Some(80),
            Self::Https => Some(443),
            _ => None,
        }
    }

    /// Retorna o nome textual do esquema em minúsculas.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
            Self::File => "file",
            Self::Custom(s) => s.as_str(),
        }
    }
}

/// Host de uma origem de rede.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Host {
    Domain(SmolStr),
    Ip(IpAddr),
    Opaque,
}

impl Host {
    /// Retorna a representação textual do host.
    pub fn as_str(&self) -> String {
        match self {
            Self::Domain(s) => s.to_string(),
            Self::Ip(ip) => ip.to_string(),
            Self::Opaque => "".to_string(),
        }
    }

    /// Retorna `true` se o host representar um loopback local (`localhost`, `127.0.0.1`, `::1`).
    pub fn is_localhost(&self) -> bool {
        match self {
            Self::Domain(s) => s == "localhost" || s.ends_with(".localhost"),
            Self::Ip(ip) => ip.is_loopback(),
            Self::Opaque => false,
        }
    }
}

/// Uma Origem Web imutável (RFC 6454 / WHATWG URL Standard).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Origin {
    /// Origem canônica em tupla `(scheme, host, port)`.
    Tuple {
        scheme: Scheme,
        host: Host,
        port: u16,
    },
    /// Origem opaca única e isolada (para `about:blank`, `data:` URLs ou iframes com sandbox).
    Opaque(u64),
}

impl Origin {
    /// Cria uma nova origem canônica a partir de esquema, host e porta.
    pub fn tuple(scheme: Scheme, host: Host, port: Option<u16>) -> Self {
        let port = port.or_else(|| scheme.default_port()).unwrap_or(0);
        Self::Tuple { scheme, host, port }
    }

    /// Gera uma nova origem opaca única garantida como disjunta de qualquer outra origem.
    pub fn new_opaque() -> Self {
        let id = OPAQUE_ORIGIN_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self::Opaque(id)
    }

    /// Retorna `true` se a origem for opaca.
    #[inline]
    pub fn is_opaque(&self) -> bool {
        matches!(self, Self::Opaque(_))
    }

    /// Retorna `true` se este contexto for considerado um Contexto Seguro (HTTPS ou Localhost).
    pub fn is_secure(&self) -> bool {
        match self {
            Self::Tuple { scheme, host, .. } => *scheme == Scheme::Https || host.is_localhost(),
            Self::Opaque(_) => false,
        }
    }

    /// Verifica se duas origens são estritamente iguais segundo a Same-Origin Policy (SOP).
    ///
    /// Duas origens opacas **nunca** são iguais entre si, garantindo sandbox completo.
    pub fn same_origin(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Tuple {
                    scheme: s1,
                    host: h1,
                    port: p1,
                },
                Self::Tuple {
                    scheme: s2,
                    host: h2,
                    port: p2,
                },
            ) => s1 == s2 && h1 == h2 && p1 == p2,
            _ => false,
        }
    }

    /// Converte esta origem em um `SchemefulSite` (esquema + eTLD+1).
    pub fn to_site(&self) -> crate::security::site::SchemefulSite {
        crate::security::site::SchemefulSite::from_origin(self)
    }

    /// Verifica se duas origens compartilham o mesmo Site (Same-Site).
    pub fn is_same_site(&self, other: &Self) -> bool {
        self.to_site().same_site(&other.to_site())
    }

    /// Converte a origem em sua serialização ASCII padrão (RFC 6454 Section 6.2).
    /// Ex: `"https://example.com:443"`, `"http://localhost:8080"` ou `"null"` para opacas.
    pub fn ascii_serialization(&self) -> String {
        match self {
            Self::Tuple { scheme, host, port } => {
                let default_port = scheme.default_port();
                if *port == 0 || default_port == Some(*port) {
                    format!("{}://{}", scheme.as_str(), host.as_str())
                } else {
                    format!("{}://{}:{}", scheme.as_str(), host.as_str(), port)
                }
            }
            Self::Opaque(_) => "null".to_string(),
        }
    }


    /// Analisa uma string de URL conforme a especificação WHATWG URL para extrair sua origem canônica.
    pub fn parse(url_str: &str) -> Result<Self, AceError> {
        let trimmed = url_str.trim();
        if trimmed.starts_with("data:") || trimmed == "about:blank" {
            return Ok(Self::new_opaque());
        }

        let parsed_url = url::Url::parse(trimmed).map_err(|e| {
            AceError::security(
                "SOP",
                format!("URL inválida para extração de origem: {}", e),
            )
        })?;

        let scheme = match parsed_url.scheme() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            "file" => Scheme::File,
            other => Scheme::Custom(SmolStr::new(other)),
        };

        let host = match parsed_url.host() {
            Some(url::Host::Domain(d)) => Host::Domain(SmolStr::new(d)),
            Some(url::Host::Ipv4(ip)) => Host::Ip(IpAddr::V4(ip)),
            Some(url::Host::Ipv6(ip)) => Host::Ip(IpAddr::V6(ip)),
            None => Host::Opaque,
        };

        let port = parsed_url.port();

        Ok(Self::tuple(scheme, host, port))
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ascii_serialization())
    }
}

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl fmt::Display for Host {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Origin {
    type Err = AceError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}
