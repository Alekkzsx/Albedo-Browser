//! # Resource Hints e Pré-aquecimento de Conexões (W3C Resource Hints)
//!
//! Otimizações especulativas para acelerar navegação e reduzir First Contentful Paint:
//! - `dns-prefetch`: Resolução de IP em background antes do clique ou requisição real
//! - `preconnect`: Abertura antecipada de socket TCP e negociação TLS no pool de conexões
//! - `preload`: Disparo antecipado de sub-recursos críticos

use crate::http::request::RequestDestination;
use ace_core::security::origin::Origin;
use url::Url;

/// Representa uma dica de otimização de recurso emitida pelo documento HTML ou scanner especulativo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceHint {
    /// Resolução DNS antecipada para o domínio informado.
    DnsPrefetch(String),
    /// Pré-conexão TCP e handshake TLS para a origem informada.
    Preconnect {
        origin: Origin,
        cross_origin: bool,
    },
    /// Pré-carregamento especulativo de um recurso específico.
    Preload {
        url: Url,
        destination: RequestDestination,
    },
}

impl ResourceHint {
    /// Cria uma dica de prefetch de DNS.
    pub fn dns_prefetch(host: impl Into<String>) -> Self {
        Self::DnsPrefetch(host.into())
    }

    /// Cria uma dica de pré-conexão para uma origem.
    pub fn preconnect(origin: Origin, cross_origin: bool) -> Self {
        Self::Preconnect { origin, cross_origin }
    }

    /// Cria uma dica de pré-carregamento com destino de recurso.
    pub fn preload(url: Url, destination: RequestDestination) -> Self {
        Self::Preload { url, destination }
    }

    /// Tenta extrair um host a partir da dica (para fins de resolução de DNS).
    pub fn target_host(&self) -> Option<smol_str::SmolStr> {
        match self {
            Self::DnsPrefetch(host) => Some(smol_str::SmolStr::new(host)),
            Self::Preconnect { origin, .. } => match origin {
                Origin::Tuple { host, .. } => Some(host.as_str()),
                Origin::Opaque { .. } => None,
            },
            Self::Preload { url, .. } => url.host_str().map(smol_str::SmolStr::new),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_hint_instantiation() {
        let hint_dns = ResourceHint::dns_prefetch("cdn.example.com");
        assert!(matches!(hint_dns, ResourceHint::DnsPrefetch(_)));
        assert_eq!(hint_dns.target_host().as_deref(), Some("cdn.example.com"));

        let origin = Origin::parse("https://fonts.googleapis.com").unwrap();
        let hint_preconnect = ResourceHint::preconnect(origin, true);
        assert!(matches!(hint_preconnect, ResourceHint::Preconnect { .. }));
        assert_eq!(hint_preconnect.target_host().as_deref(), Some("fonts.googleapis.com"));

        let url = Url::parse("https://example.com/style.css").unwrap();
        let hint_preload = ResourceHint::preload(url, RequestDestination::Style);
        assert_eq!(hint_preload.target_host().as_deref(), Some("example.com"));
    }
}

