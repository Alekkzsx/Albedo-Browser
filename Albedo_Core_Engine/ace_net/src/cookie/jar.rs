//! # Gerenciador de Armazenamento de Cookies (`CookieJar`)
//!
//! Gerenciador centralizado e thread-safe de cookies com suporte a atualização,
//! expiração, higienização e particionamento CHIPS.

use super::entry::Cookie;
use crate::request::CredentialsMode;
use http::header::{SET_COOKIE};
use http::{HeaderMap, HeaderValue};
use parking_lot::RwLock;
use std::time::SystemTime;
use url::Url;

/// Armazenamento em memória de cookies do navegador.
#[derive(Debug, Default)]
pub struct CookieJar {
    cookies: RwLock<Vec<Cookie>>,
}

impl CookieJar {
    /// Cria uma nova instância de `CookieJar`.
    pub fn new() -> Self {
        Self {
            cookies: RwLock::new(Vec::new()),
        }
    }

    /// Armazena ou atualiza um cookie no pote. Se já existir (mesmo nome, domínio, path e partition_key), substitui.
    pub fn store_cookie(&self, new_cookie: Cookie) {
        let mut list = self.cookies.write();
        let mut found = false;

        for existing in list.iter_mut() {
            if existing.name == new_cookie.name
                && existing.domain == new_cookie.domain
                && existing.path == new_cookie.path
                && existing.partition_key == new_cookie.partition_key
            {
                *existing = new_cookie.clone();
                found = true;
                break;
            }
        }

        if !found {
            list.push(new_cookie);
        }
    }

    /// Extrai e processa todos os cabeçalhos `Set-Cookie` retornados em uma resposta HTTP.
    pub fn process_response_headers(
        &self,
        url: &Url,
        headers: &HeaderMap,
        top_level_site: Option<&str>,
        now: SystemTime,
    ) {
        for val in headers.get_all(SET_COOKIE) {
            if let Ok(s) = val.to_str() {
                if let Some(cookie) = Cookie::parse(s, url, top_level_site, now) {
                    self.store_cookie(cookie);
                }
            }
        }
    }

    /// Monta o cabeçalho `Cookie: name1=val1; name2=val2` para ser enviado na requisição.
    pub fn build_cookie_header(
        &self,
        url: &Url,
        top_level_site: Option<&str>,
        credentials_mode: CredentialsMode,
        is_same_site: bool,
        is_navigation_get: bool,
        now: SystemTime,
    ) -> Option<HeaderValue> {
        let list = self.cookies.read();
        let mut matching_cookies = Vec::new();

        for cookie in list.iter() {
            if cookie.is_valid_for_request(
                url,
                top_level_site,
                credentials_mode,
                is_same_site,
                is_navigation_get,
                now,
            ) {
                matching_cookies.push(format!("{}={}", cookie.name, cookie.value));
            }
        }

        if matching_cookies.is_empty() {
            None
        } else {
            let combined = matching_cookies.join("; ");
            HeaderValue::from_str(&combined).ok()
        }
    }

    /// Remove todos os cookies vinculados ao domínio especificado (ou seus subdomínios).
    pub fn clear_for_domain(&self, domain: &str) {
        let clean = domain.trim_start_matches('.').to_ascii_lowercase();
        let mut list = self.cookies.write();
        list.retain(|c| c.domain != clean && !c.domain.ends_with(&format!(".{}", clean)));
    }

    /// Remove todos os cookies do jar.
    pub fn clear_all(&self) {
        self.cookies.write().clear();
    }

    /// Retorna a contagem atual de cookies ativos.
    pub fn len(&self) -> usize {
        self.cookies.read().len()
    }

    /// Verifica se o jar está vazio.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_jar_basic_flow() {
        let jar = CookieJar::new();
        let url = Url::parse("https://example.com/app/index.html").unwrap();
        let now = SystemTime::now();

        let mut headers = HeaderMap::new();
        headers.append(
            SET_COOKIE,
            HeaderValue::from_static("session_id=abc12345; Secure; HttpOnly; SameSite=Strict"),
        );
        headers.append(
            SET_COOKIE,
            HeaderValue::from_static("theme=dark; Path=/app; Max-Age=3600"),
        );

        jar.process_response_headers(&url, &headers, None, now);
        assert_eq!(jar.len(), 2);

        let cookie_hdr = jar
            .build_cookie_header(&url, None, CredentialsMode::SameOrigin, true, true, now)
            .unwrap();

        let s = cookie_hdr.to_str().unwrap();
        assert!(s.contains("session_id=abc12345"));
        assert!(s.contains("theme=dark"));
    }

    #[test]
    fn test_cookie_chips_partitioning() {
        let jar = CookieJar::new();
        let target_url = Url::parse("https://widget.example/embed.js").unwrap();
        let now = SystemTime::now();

        // Cookie emitido quando embutido em site-a.com
        let cookie_a = Cookie::parse(
            "user_pref=blue; Secure; Partitioned; SameSite=None",
            &target_url,
            Some("site-a.com"),
            now,
        )
        .unwrap();
        assert_eq!(cookie_a.partition_key.as_deref(), Some("site-a.com"));
        jar.store_cookie(cookie_a);

        // Quando requisitado por site-a.com, deve enviar
        let hdr_a = jar.build_cookie_header(
            &target_url,
            Some("site-a.com"),
            CredentialsMode::Include,
            false,
            false,
            now,
        );
        assert!(hdr_a.is_some());

        // Quando requisitado por site-b.com, NÃO deve enviar (CHIPS isolation)
        let hdr_b = jar.build_cookie_header(
            &target_url,
            Some("site-b.com"),
            CredentialsMode::Include,
            false,
            false,
            now,
        );
        assert!(hdr_b.is_none());
    }
}
