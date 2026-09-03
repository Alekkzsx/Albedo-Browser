//! # Resource Hints e Pré-aquecimento de Conexões (W3C Resource Hints)
//!
//! Otimizações especulativas para acelerar navegação e reduzir First Contentful Paint:
//! - `dns-prefetch`: Resolução de IP em background antes do clique ou requisição real
//! - `preconnect`: Abertura antecipada de socket TCP e negociação TLS no pool de conexões
//! - `preload`: Disparo antecipado de sub-recursos críticos

use crate::request::RequestDestination;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_hint_instantiation() {
        let hint_dns = ResourceHint::DnsPrefetch("cdn.example.com".to_string());
        assert!(matches!(hint_dns, ResourceHint::DnsPrefetch(_)));

        let hint_preconnect = ResourceHint::Preconnect {
            origin: Origin::parse("https://fonts.googleapis.com").unwrap(),
            cross_origin: true,
        };
        assert!(matches!(hint_preconnect, ResourceHint::Preconnect { .. }));
    }
}
