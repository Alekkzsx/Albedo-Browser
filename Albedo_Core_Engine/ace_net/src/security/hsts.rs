//! # HTTP Strict Transport Security (HSTS - RFC 6797) & Preload List
//!
//! Previne ataques de Man-in-the-Middle (MITM) e SSL Stripping garantindo que
//! conexões para domínios seguros ocorram exclusivamente via HTTPS, com suporte a
//! auto-upgrade transparente em memória e lista precarregada (HSTS Preload List).

use http::HeaderValue;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use std::time::{Duration, SystemTime};
use url::Url;

/// Política de segurança HSTS registrada para um domínio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HstsPolicy {
    pub expires_at: SystemTime,
    pub include_subdomains: bool,
}

impl HstsPolicy {
    pub fn is_fresh(&self, now: SystemTime) -> bool {
        self.expires_at > now
    }
}

/// Lista hardcoded de domínios pré-carregados (HSTS Preload List) que exigem HTTPS desde a primeira conexão.
const PRELOADED_HSTS_DOMAINS: &[(&str, bool)] = &[
    ("google.com", true),
    ("github.com", true),
    ("cloudflare.com", true),
    ("wikipedia.org", true),
    ("mozilla.org", true),
    ("rust-lang.org", true),
    ("duckduckgo.com", true),
    ("albedo.browser", true),
];

/// Verifica se `host` é um subdomínio válido de `parent` sem alocação dinâmica.
#[inline]
fn is_subdomain_of(host: &str, parent: &str) -> bool {
    if host.len() > parent.len() && host.ends_with(parent) {
        host.as_bytes()[host.len() - parent.len() - 1] == b'.'
    } else {
        false
    }
}

/// Registro compartilhado em memória de políticas HSTS ativas.
#[derive(Debug, Default)]
pub struct HstsStore {
    entries: RwLock<FxHashMap<String, HstsPolicy>>,
}

impl HstsStore {
    /// Cria uma nova instância de `HstsStore`.
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(FxHashMap::default()),
        }
    }

    /// Atualiza ou insere a política HSTS a partir do cabeçalho `Strict-Transport-Security`.
    pub fn update_from_header(&self, host: &str, header_val: &HeaderValue, now: SystemTime) {
        let s = match header_val.to_str() {
            Ok(v) => v,
            Err(_) => return,
        };

        let mut max_age = None;
        let mut include_subdomains = false;

        for part in s.split(';') {
            let part = part.trim();
            if let Some(stripped) = part.strip_prefix("max-age=") {
                if let Ok(secs) = stripped.trim().parse::<u64>() {
                    max_age = Some(Duration::from_secs(secs));
                }
            } else if part.eq_ignore_ascii_case("includesubdomains") {
                include_subdomains = true;
            }
        }

        if let Some(dur) = max_age {
            let clean_host = host.to_ascii_lowercase();
            let mut map = self.entries.write();

            if dur.is_zero() {
                // max-age=0 revoga a política
                map.remove(&clean_host);
            } else {
                map.insert(
                    clean_host,
                    HstsPolicy {
                        expires_at: now + dur,
                        include_subdomains,
                    },
                );
            }
        }
    }

    /// Determina se um host deve ser atualizado obrigatoriamente para HTTPS.
    pub fn should_upgrade(&self, host: &str, now: SystemTime) -> bool {
        let clean_host = host.to_ascii_lowercase();

        // 1. Checa HSTS Preload List (zero alocação)
        for &(domain, include_sub) in PRELOADED_HSTS_DOMAINS {
            if clean_host == domain {
                return true;
            }
            if include_sub && is_subdomain_of(&clean_host, domain) {
                return true;
            }
        }

        // 2. Checa HSTS dinâmico em memória com parent-domain walking O(depth)
        let map = self.entries.read();

        // Correspondência exata do host
        if let Some(policy) = map.get(&clean_host) {
            if policy.is_fresh(now) {
                return true;
            }
        }

        // Caminhamento de domínios ancestrais (ex: sub.example.com -> example.com)
        for (dot_idx, _) in clean_host.match_indices('.') {
            let parent = &clean_host[dot_idx + 1..];
            if let Some(policy) = map.get(parent) {
                if policy.include_subdomains && policy.is_fresh(now) {
                    return true;
                }
            }
        }

        false
    }

    /// Se a URL for HTTP e o domínio exigir HSTS, reescreve a URL para HTTPS em memória.
    pub fn upgrade_url(&self, url: &Url, now: SystemTime) -> Option<Url> {
        if url.scheme() != "http" {
            return None;
        }

        let host = url.host_str()?;
        if self.should_upgrade(host, now) {
            let mut new_url = url.clone();
            let _ = new_url.set_scheme("https");
            // Se porta explícita for 80, remove para usar 443 padrão
            if new_url.port() == Some(80) {
                let _ = new_url.set_port(None);
            }
            Some(new_url)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hsts_preload_list_upgrade() {
        let store = HstsStore::new();
        let now = SystemTime::now();

        let http_url = Url::parse("http://github.com/explore").unwrap();
        let upgraded = store.upgrade_url(&http_url, now).unwrap();
        assert_eq!(upgraded.as_str(), "https://github.com/explore");

        let sub_url = Url::parse("http://api.github.com/users").unwrap();
        let upgraded_sub = store.upgrade_url(&sub_url, now).unwrap();
        assert_eq!(upgraded_sub.as_str(), "https://api.github.com/users");
    }

    #[test]
    fn test_hsts_dynamic_header_and_expiry() {
        let store = HstsStore::new();
        let now = SystemTime::now();

        let val = HeaderValue::from_static("max-age=3600; includeSubDomains");
        store.update_from_header("example.com", &val, now);

        assert!(store.should_upgrade("example.com", now));
        assert!(store.should_upgrade("sub.example.com", now));

        // Após expirar
        let future = now + Duration::from_secs(3601);
        assert!(!store.should_upgrade("example.com", future));
    }
}
