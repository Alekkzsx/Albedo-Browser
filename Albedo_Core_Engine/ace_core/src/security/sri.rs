//! # Validação de Integridade de Sub-recursos e Hasher CSP3 (W3C SRI)
//!
//! Implementação de streaming incremental de integridade criptográfica (SHA-256 / SHA-384 / SHA-512)
//! para validar scripts e folhas de estilo baixadas pela rede antes da entrega ao motor de renderização.
//! Segue a regra W3C de priorização estrita para o algoritmo mais forte e comparação em tempo constante.

use sha2::{Digest, Sha256, Sha384, Sha512};
use std::fmt;

/// Algoritmos criptográficos de integridade suportados pelo padrão W3C SRI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SriAlgorithm {
    Sha256 = 1,
    Sha384 = 2,
    Sha512 = 3,
}

impl SriAlgorithm {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "sha256" => Some(Self::Sha256),
            "sha384" => Some(Self::Sha384),
            "sha512" => Some(Self::Sha512),
            _ => None,
        }
    }
}

/// Metadados de integridade extraídos do atributo `integrity`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SriMetadata {
    pub algorithm: SriAlgorithm,
    pub raw_digest: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SriError {
    NoValidAlgorithms,
    DigestMismatch,
}

impl fmt::Display for SriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for SriError {}

/// Hasher incremental para validação de dados em streaming conforme chegam do socket de rede.
pub struct StreamingSriHasher {
    algorithm: SriAlgorithm,
    target_digest: Vec<u8>,
    ctx_256: Option<Sha256>,
    ctx_384: Option<Sha384>,
    ctx_512: Option<Sha512>,
}

impl StreamingSriHasher {
    /// Cria um novo hasher para o algoritmo e digest esperado.
    pub fn new(algorithm: SriAlgorithm, target_digest: Vec<u8>) -> Self {
        let (ctx_256, ctx_384, ctx_512) = match algorithm {
            SriAlgorithm::Sha256 => (Some(Sha256::new()), None, None),
            SriAlgorithm::Sha384 => (None, Some(Sha384::new()), None),
            SriAlgorithm::Sha512 => (None, None, Some(Sha512::new())),
        };

        Self {
            algorithm,
            target_digest,
            ctx_256,
            ctx_384,
            ctx_512,
        }
    }

    /// Alimenta fatias do payload incrementalmente conforme chegam da rede.
    pub fn update(&mut self, chunk: &[u8]) {
        if let Some(h) = &mut self.ctx_256 {
            h.update(chunk);
        } else if let Some(h) = &mut self.ctx_384 {
            h.update(chunk);
        } else if let Some(h) = &mut self.ctx_512 {
            h.update(chunk);
        }
    }

    /// Comparação em tempo estritamente constante contra ataques de canal lateral (Timing Attacks).
    fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        let mut diff = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            diff |= x ^ y;
        }
        diff == 0
    }

    /// Finaliza o cálculo do digest e valida contra o esperado.
    pub fn finalize_and_verify(self) -> Result<(), SriError> {
        let computed: Vec<u8> = if let Some(h) = self.ctx_256 {
            h.finalize().to_vec()
        } else if let Some(h) = self.ctx_384 {
            h.finalize().to_vec()
        } else if let Some(h) = self.ctx_512 {
            h.finalize().to_vec()
        } else {
            return Err(SriError::NoValidAlgorithms);
        };

        if Self::constant_time_eq(&computed, &self.target_digest) {
            Ok(())
        } else {
            Err(SriError::DigestMismatch)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sri_streaming_sha256() {
        let data = b"console.log('Hello Albedo SRI!');";
        let mut hasher = Sha256::new();
        hasher.update(data);
        let expected_digest = hasher.finalize().to_vec();

        let mut streaming =
            StreamingSriHasher::new(SriAlgorithm::Sha256, expected_digest.clone());

        streaming.update(&data[0..10]);
        streaming.update(&data[10..20]);
        streaming.update(&data[20..]);

        assert!(streaming.finalize_and_verify().is_ok());
    }
}
