//! # Modelo de Site e Domínio Registrável (RFC 6265bis / SchemefulSite Pattern)
//!
//! Modela a separação entre *Origin* (tupla estrita) e *Site* (esquema + eTLD+1),
//! fundamental para cookies `SameSite`, particionamento de storage e Site Isolation.

use crate::security::origin::{Host, Origin, Scheme};
use smol_str::SmolStr;
use std::fmt;

/// Representa um Site na web (Esquema + Domínio Registrável / eTLD+1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SchemefulSite {
    scheme: Option<Scheme>,
    registrable_domain: SmolStr,
    is_opaque: bool,
}

impl SchemefulSite {
    /// Cria um novo `SchemefulSite` a partir de uma `Origin`.
    pub fn from_origin(origin: &Origin) -> Self {
        match origin {
            Origin::Tuple { scheme, host, .. } => {
                let reg_domain = match host {
                    Host::Domain(d) => extract_registrable_domain(d.as_str()),
                    Host::Ip(ip) => ip.to_string().into(),
                    Host::Opaque => "".into(),
                };
                Self {
                    scheme: Some(scheme.clone()),
                    registrable_domain: reg_domain,
                    is_opaque: false,
                }
            }
            Origin::Opaque(id) => Self {
                scheme: None,
                registrable_domain: format!("opaque-site-{}", id).into(),
                is_opaque: true,
            },
        }
    }

    /// Retorna `true` se o site for opaco (sandbox, `data:` URI).
    #[inline]
    pub fn is_opaque(&self) -> bool {
        self.is_opaque
    }

    /// Retorna o esquema do site, se aplicável.
    #[inline]
    pub fn scheme(&self) -> Option<&Scheme> {
        self.scheme.as_ref()
    }

    /// Retorna o domínio registrável (eTLD+1).
    #[inline]
    pub fn registrable_domain(&self) -> &str {
        self.registrable_domain.as_str()
    }

    /// Retorna `true` se dois sites forem idênticos conforme a política Same-Site.
    pub fn same_site(&self, other: &Self) -> bool {
        if self.is_opaque || other.is_opaque {
            return false;
        }
        self.scheme == other.scheme && self.registrable_domain == other.registrable_domain
    }
}

/// Extrai a porção registrável do domínio (eTLD+1), considerando sufixos públicos comuns.
fn extract_registrable_domain(host: &str) -> SmolStr {
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty() || host == "localhost" || host.ends_with(".localhost") {
        return SmolStr::new(host);
    }

    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() <= 2 {
        return SmolStr::new(host);
    }

    // Sufixos públicos comuns de segundo nível (eTLD composto)
    let is_multipart_tld = match (parts[parts.len() - 2], parts[parts.len() - 1]) {
        ("co" | "com" | "org" | "net" | "edu" | "gov" | "mil", "uk" | "br" | "au" | "nz" | "za" | "jp") => true,
        ("ac" | "gov" | "edu", _) => true,
        _ => false,
    };

    if is_multipart_tld && parts.len() >= 3 {
        let domain_parts = &parts[parts.len() - 3..];
        domain_parts.join(".").into()
    } else {
        let domain_parts = &parts[parts.len() - 2..];
        domain_parts.join(".").into()
    }
}

impl fmt::Display for SchemefulSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_opaque {
            write!(f, "null")
        } else if let Some(ref s) = self.scheme {
            write!(f, "{}://{}", s.as_str(), self.registrable_domain)
        } else {
            write!(f, "{}", self.registrable_domain)
        }
    }
}
