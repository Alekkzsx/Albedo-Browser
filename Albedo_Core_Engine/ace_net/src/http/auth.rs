//! # Gerenciador de Autenticação HTTP e Desafios 401 (RFC 7235 / RFC 7617)
//!
//! Implementa o tratamento normativo de desafios `401 Unauthorized`:
//! - Parsing estruturado de cabeçalhos `WWW-Authenticate: Basic / Bearer realm="..."`
//! - Cache de credenciais em memória (`HttpAuthCache`) indexado por `(Origin, Realm)`
//! - Geração de cabeçalhos de autorização `Authorization: Basic <base64>` para re-tentativas automáticas

use base64::Engine;
use http::header::WWW_AUTHENTICATE;
use http::HeaderMap;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::sync::Arc;

/// Desafio de autenticação extraído do cabeçalho `WWW-Authenticate` (RFC 7235 §4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpAuthChallenge {
    /// Esquema de autenticação (ex: "Basic", "Bearer", "Digest").
    pub scheme: SmolStr,
    /// Domínio de proteção ou reino (realm) de autorização.
    pub realm: SmolStr,
    /// Parâmetros adicionais da diretiva (ex: `charset="UTF-8"`, `error="invalid_token"`).
    pub params: Vec<(SmolStr, SmolStr)>,
}

impl HttpAuthChallenge {
    /// Analisa uma string do cabeçalho `WWW-Authenticate`.
    pub fn parse(header_str: &str) -> Option<Self> {
        let trimmed = header_str.trim();
        let mut parts = trimmed.splitn(2, ' ');
        let scheme = parts.next()?.trim();
        if scheme.is_empty() {
            return None;
        }

        let raw_params = parts.next().unwrap_or("").trim();
        let mut realm = SmolStr::default();
        let mut params = Vec::new();

        for item in raw_params.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            if let Some((k, v)) = item.split_once('=') {
                let k_clean = SmolStr::new(k.trim().to_ascii_lowercase());
                let v_clean = SmolStr::new(v.trim().trim_matches('"'));

                if k_clean == "realm" {
                    realm = v_clean.clone();
                }
                params.push((k_clean, v_clean));
            }
        }

        Some(Self {
            scheme: SmolStr::new(scheme),
            realm,
            params,
        })
    }
}

/// Credenciais de autenticação HTTP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpAuthCredentials {
    pub username: SmolStr,
    pub password: SmolStr,
}

impl HttpAuthCredentials {
    pub fn new(username: impl Into<SmolStr>, password: impl Into<SmolStr>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Codifica as credenciais para o formato canônico `Basic <base64(user:pass)>` (RFC 7617).
    pub fn to_basic_header_value(&self) -> String {
        let raw = format!("{}:{}", self.username, self.password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(raw);
        format!("Basic {}", encoded)
    }
}

/// Cache de credenciais de autenticação indexado por tupla `(Origin, Realm)`.
#[derive(Debug, Clone, Default)]
pub struct HttpAuthCache {
    entries: Arc<RwLock<FxHashMap<(String, SmolStr), HttpAuthCredentials>>>,
}

impl HttpAuthCache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }

    /// Armazena credenciais em cache para uma dada origem e realm.
    pub fn insert(&self, origin: &str, realm: &str, creds: HttpAuthCredentials) {
        let key = (origin.to_string(), SmolStr::new(realm));
        self.entries.write().insert(key, creds);
    }

    /// Obtém credenciais em cache para uma origem e realm específicos.
    pub fn get(&self, origin: &str, realm: &str) -> Option<HttpAuthCredentials> {
        let key = (origin.to_string(), SmolStr::new(realm));
        self.entries.read().get(&key).cloned()
    }

    /// Limpa todas as credenciais em cache.
    pub fn clear(&self) {
        self.entries.write().clear();
    }
}

/// Gerenciador unificado de autenticação HTTP para o motor de rede.
#[derive(Debug, Clone, Default)]
pub struct HttpAuthManager {
    cache: HttpAuthCache,
}

impl HttpAuthManager {
    pub fn new() -> Self {
        Self {
            cache: HttpAuthCache::new(),
        }
    }

    /// Retorna uma referência ao cache de credenciais subjacente.
    pub fn cache(&self) -> &HttpAuthCache {
        &self.cache
    }

    /// Extrai o desafio do cabeçalho `WWW-Authenticate` presente nos cabeçalhos de resposta HTTP.
    pub fn extract_challenge(headers: &HeaderMap) -> Option<HttpAuthChallenge> {
        let val = headers.get(WWW_AUTHENTICATE)?.to_str().ok()?;
        HttpAuthChallenge::parse(val)
    }

    /// Gera o cabeçalho `Authorization` adequado a partir do cache caso haja credenciais conhecidas.
    pub fn resolve_authorization_header(&self, origin: &str, challenge: &HttpAuthChallenge) -> Option<String> {
        if challenge.scheme.eq_ignore_ascii_case("basic") {
            let creds = self.cache.get(origin, challenge.realm.as_str())?;
            Some(creds.to_basic_header_value())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_www_authenticate_basic() {
        let header = "Basic realm=\"Staging Admin\", charset=\"UTF-8\"";
        let challenge = HttpAuthChallenge::parse(header).expect("Deve analisar desafio Basic");

        assert_eq!(challenge.scheme, "Basic");
        assert_eq!(challenge.realm, "Staging Admin");
        assert_eq!(challenge.params.len(), 2);
    }

    #[test]
    fn test_auth_cache_and_authorization_resolution() {
        let manager = HttpAuthManager::new();
        let origin = "https://internal.corp";
        let realm = "Restricted Area";

        manager.cache().insert(origin, realm, HttpAuthCredentials::new("alice", "p@ssword123"));

        let challenge = HttpAuthChallenge {
            scheme: SmolStr::new("Basic"),
            realm: SmolStr::new(realm),
            params: vec![],
        };

        let auth_hdr = manager.resolve_authorization_header(origin, &challenge);
        assert!(auth_hdr.is_some());
        let val = auth_hdr.unwrap();
        assert!(val.starts_with("Basic "));
        // "alice:p@ssword123" em base64 é "YWxpY2U6cEBzc3dvcmQxMjM="
        assert_eq!(val, "Basic YWxpY2U6cEBzc3dvcmQxMjM=");
    }
}

