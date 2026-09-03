//! # Particionamento de Cache por Network Isolation Key (NIK)
//!
//! Evita vazamentos de histórico, ataques de canal lateral e rastreamento cross-site
//! particionando as chaves de cache por (TopFrameOrigin, FrameOrigin), conforme
//! a especificação de Network State Partitioning do W3C/Chromium.

use ace_core::security::origin::Origin;
use std::fmt;

/// Chave de isolamento de rede para particionamento de estado (HTTP Cache, conexões).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkIsolationKey {
    /// Origem do frame de nível superior (Top-Level Document).
    pub top_frame_origin: Origin,
    /// Origem do frame imediato que disparou o recurso.
    pub frame_origin: Origin,
}

impl NetworkIsolationKey {
    /// Cria uma nova chave de isolamento de rede.
    pub fn new(top_frame_origin: Origin, frame_origin: Origin) -> Self {
        Self {
            top_frame_origin,
            frame_origin,
        }
    }

    /// Cria uma chave para contexto de navegação primária (onde o frame é o próprio topo).
    pub fn for_top_level(origin: Origin) -> Self {
        Self {
            top_frame_origin: origin.clone(),
            frame_origin: origin,
        }
    }

    /// Serializa a NIK para inclusão determinística em chaves compostas de cache.
    pub fn serialize(&self) -> String {
        format!("{}^{}", self.top_frame_origin.ascii_serialization(), self.frame_origin.ascii_serialization())
    }
}

impl fmt::Display for NetworkIsolationKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.serialize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nik_serialization_and_isolation() {
        let top = Origin::parse("https://example.com").unwrap();
        let frame = Origin::parse("https://cdn.example.com").unwrap();
        let other = Origin::parse("https://malicious.org").unwrap();

        let nik1 = NetworkIsolationKey::new(top.clone(), frame.clone());
        let nik2 = NetworkIsolationKey::new(other, frame.clone());

        assert_ne!(nik1, nik2);
        assert_eq!(nik1.serialize(), "https://example.com^https://cdn.example.com");
    }
}
