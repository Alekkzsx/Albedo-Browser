//! # Subresource Integrity (W3C SRI - RFC 9110 / W3C Recommendation)
//!
//! Permite que navegadores verifiquem a integridade criptográfica de arquivos
//! obtidos de terceiros (CDNs, scripts, stylesheets), garantindo que conteúdos
//! modificados ou maliciosos nunca sejam executados pelo motor.

use crate::error::{NetError, NetResult};
use base64::prelude::*;
use sha2::{Digest, Sha256, Sha384, Sha512};

/// Algoritmos de hash criptográficos suportados pelo W3C SRI.
/// Ordenados do mais fraco ao mais forte (Sha512 > Sha384 > Sha256).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SriAlgorithm {
    Sha256 = 1,
    Sha384 = 2,
    Sha512 = 3,
}

impl SriAlgorithm {
    pub fn from_prefix(prefix: &str) -> Option<Self> {
        match prefix.to_ascii_lowercase().as_str() {
            "sha256" => Some(Self::Sha256),
            "sha384" => Some(Self::Sha384),
            "sha512" => Some(Self::Sha512),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sha256 => "sha256",
            Self::Sha384 => "sha384",
            Self::Sha512 => "sha512",
        }
    }
}

/// Metadado de integridade parsed de um item da especificação SRI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SriDigest {
    pub algorithm: SriAlgorithm,
    pub digest_bytes: Vec<u8>,
    pub raw_base64: String,
}

/// Faz o parsing do atributo HTML `integrity` (ex: `sha384-xyz... sha512-abc...`).
/// Itens com algoritmos desconhecidos ou formato inválido são ignorados conforme W3C SRI §3.3.3.
pub fn parse_integrity(attr: &str) -> Vec<SriDigest> {
    let mut results = Vec::new();

    for token in attr.split_whitespace() {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }

        // Separa no hífen entre o algoritmo e o base64 (ex: "sha384-xyz?opt" -> ("sha384", "xyz?opt"))
        if let Some((algo_part, b64_and_opts)) = token.split_once('-') {
            if let Some(algo) = SriAlgorithm::from_prefix(algo_part) {
                // Remove quaisquer opções "?..." anexadas ao base64
                let b64_str = b64_and_opts.split('?').next().unwrap_or("").trim();
                if let Ok(bytes) = BASE64_STANDARD.decode(b64_str) {
                    // Valida o tamanho esperado do digest
                    let expected_len = match algo {
                        SriAlgorithm::Sha256 => 32,
                        SriAlgorithm::Sha384 => 48,
                        SriAlgorithm::Sha512 => 64,
                    };
                    if bytes.len() == expected_len {
                        results.push(SriDigest {
                            algorithm: algo,
                            digest_bytes: bytes,
                            raw_base64: b64_str.to_string(),
                        });
                    }
                }
            }
        }
    }

    results
}

/// Calcula o hash criptográfico de um payload utilizando o algoritmo especificado.
pub fn compute_hash(algo: SriAlgorithm, payload: &[u8]) -> Vec<u8> {
    match algo {
        SriAlgorithm::Sha256 => {
            let mut hasher = Sha256::new();
            hasher.update(payload);
            hasher.finalize().to_vec()
        }
        SriAlgorithm::Sha384 => {
            let mut hasher = Sha384::new();
            hasher.update(payload);
            hasher.finalize().to_vec()
        }
        SriAlgorithm::Sha512 => {
            let mut hasher = Sha512::new();
            hasher.update(payload);
            hasher.finalize().to_vec()
        }
    }
}

/// Converte um payload em uma string SRI válida (ex: "sha256-...").
pub fn generate_sri_hash(algo: SriAlgorithm, payload: &[u8]) -> String {
    let hash = compute_hash(algo, payload);
    let b64 = BASE64_STANDARD.encode(hash);
    format!("{}-{}", algo.as_str(), b64)
}

/// Valida a integridade do corpo da resposta segundo a especificação W3C Subresource Integrity §3.3.5.
///
/// 1. Filtra os metadados para encontrar o algoritmo mais forte presente (Sha512 > Sha384 > Sha256).
/// 2. Calcula o hash do payload usando esse algoritmo mais forte.
/// 3. Se pelo menos um dos hashes desse algoritmo mais forte corresponder ao payload, a validação tem sucesso.
/// 4. Se nenhum corresponder, retorna `NetError::SriMismatch`.
pub fn verify_integrity(payload: &[u8], digests: &[SriDigest]) -> NetResult<()> {
    if digests.is_empty() {
        return Ok(());
    }

    // Identifica o algoritmo mais forte presente
    let strongest_algo = match digests.iter().map(|d| d.algorithm).max() {
        Some(algo) => algo,
        None => return Ok(()),
    };

    let computed = compute_hash(strongest_algo, payload);

    // Dentre os digests que utilizam o algoritmo mais forte, ao menos um deve bater
    let matched = digests
        .iter()
        .filter(|d| d.algorithm == strongest_algo)
        .any(|d| d.digest_bytes == computed);

    if matched {
        Ok(())
    } else {
        let computed_b64 = BASE64_STANDARD.encode(&computed);
        let expected_list: Vec<String> = digests
            .iter()
            .filter(|d| d.algorithm == strongest_algo)
            .map(|d| format!("{}-{}", d.algorithm.as_str(), d.raw_base64))
            .collect();

        crate::telemetry::net_log::log_net_event(
            crate::telemetry::net_log::NetEventType::Error,
            "",
            &format!(
                "SRI mismatch: expected one of {:?}, got {}-{}",
                expected_list,
                strongest_algo.as_str(),
                computed_b64
            ),
        );

        Err(NetError::SriMismatch(format!(
            "Subresource Integrity check failed for algorithm {}: computed {}-{}, expected one of {:?}",
            strongest_algo.as_str(),
            strongest_algo.as_str(),
            computed_b64,
            expected_list
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sri_sha256_verification_success() {
        let data = b"console.log('hello world');";
        let expected_sri = generate_sri_hash(SriAlgorithm::Sha256, data);

        let parsed = parse_integrity(&expected_sri);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].algorithm, SriAlgorithm::Sha256);

        assert!(verify_integrity(data, &parsed).is_ok());
    }

    #[test]
    fn test_sri_tampered_payload_fails() {
        let data = b"console.log('hello world');";
        let tampered = b"console.log('hacked world');";
        let sri_str = generate_sri_hash(SriAlgorithm::Sha256, data);

        let parsed = parse_integrity(&sri_str);
        let result = verify_integrity(tampered, &parsed);

        assert!(matches!(result, Err(NetError::SriMismatch(_))));
    }

    #[test]
    fn test_sri_algorithm_priority_sha384_over_sha256() {
        let data = b"alert(1);";
        let good_sha384 = generate_sri_hash(SriAlgorithm::Sha384, data);
        let bad_sha256 = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

        // Atributo com SHA-256 errado mas SHA-384 correto
        let combined = format!("{} {}", bad_sha256, good_sha384);
        let parsed = parse_integrity(&combined);

        // Como SHA-384 é mais forte que SHA-256, o UA DEVE ignorar o SHA-256 e validar apenas o SHA-384
        assert!(verify_integrity(data, &parsed).is_ok());

        // Se o SHA-384 estiver adulterado, mesmo que o SHA-256 estivesse certo, falha
        let bad_sha384 = "sha384-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let good_sha256 = generate_sri_hash(SriAlgorithm::Sha256, data);
        let combined2 = format!("{} {}", good_sha256, bad_sha384);
        let parsed2 = parse_integrity(&combined2);
        assert!(verify_integrity(data, &parsed2).is_err());
    }
}
