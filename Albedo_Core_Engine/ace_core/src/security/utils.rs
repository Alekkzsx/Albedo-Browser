//! # Utilitários de Segurança e Políticas de Origem
//!
//! Funções para correspondência de padrões de domínio (wildcards para CSP/CORS) e validação de contextos confiáveis.

use super::Origin;

/// Verifica se um host corresponde a um padrão de domínio (com suporte a wildcards `*.domain.com`).
pub fn matches_domain_pattern(pattern: &str, host: &str) -> bool {
    let p = pattern.trim().to_ascii_lowercase();
    let h = host.trim().to_ascii_lowercase();

    if p == h || p == "*" {
        return true;
    }

    if let Some(suffix) = p.strip_prefix("*.") {
        if h == suffix {
            return true;
        }
        if h.ends_with(&format!(".{}", suffix)) {
            return true;
        }
    }

    false
}

/// Determina se uma origem é considerada "Potencialmente Confiável" (W3C Secure Contexts).
#[inline]
pub fn is_potentially_trustworthy_origin(origin: &Origin) -> bool {
    origin.is_secure()
}
