//! # Utilitários de Segurança e Políticas de Origem
//!
//! Funções para correspondência de padrões de domínio (wildcards para CSP/CORS) e validação de contextos confiáveis.

use super::Origin;

/// Verifica se um host corresponde a um padrão de domínio (com suporte a wildcards `*.domain.com` conforme W3C CSP3 §6.7.2).
///
/// Wildcards `*.example.com` casam exclusivamente com subdomínios (`sub.example.com`, `a.b.example.com`)
/// e NUNCA com o apex domain (`example.com`). Zero heap allocations.
pub fn matches_domain_pattern(pattern: &str, host: &str) -> bool {
    let p = pattern.trim();
    let h = host.trim();

    if p == "*" || p.eq_ignore_ascii_case(h) {
        return true;
    }

    if let Some(suffix) = p.strip_prefix("*.") {
        if h.len() > suffix.len() + 1 && h[h.len() - suffix.len()..].eq_ignore_ascii_case(suffix) {
            let dot_index = h.len() - suffix.len() - 1;
            if h.as_bytes()[dot_index] == b'.' {
                return true;
            }
        }
    }

    false
}


/// Determina se uma origem é considerada "Potencialmente Confiável" (W3C Secure Contexts).
#[inline]
pub fn is_potentially_trustworthy_origin(origin: &Origin) -> bool {
    origin.is_secure()
}
