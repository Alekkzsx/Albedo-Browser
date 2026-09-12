//! # Resolução Segura de Redirecionamentos HTTP (3xx)
//!
//! Implementação das diretrizes do WHATWG Fetch §4.4 (HTTP-redirect fetch):
//! - Conversão normativa de métodos HTTP (301, 302, 303 -> GET)
//! - Detecção de ciclos e limite de saltos (máximo 20)
//! - Descarte (stripping) de cabeçalhos confidenciais (`Authorization`, `Cookie`) em travessias cross-origin
//! - Resolução de URLs relativas no cabeçalho `Location`

use crate::error::{NetError, NetResult};
use crate::request::Request;
use ace_core::security::origin::Origin;
use bytes::Bytes;
use http::header::{HeaderMap, AUTHORIZATION, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, COOKIE, LOCATION};
use http::{Method, StatusCode};
use std::collections::HashSet;
use url::Url;

/// Dados da requisição após o salto de redirecionamento.
#[derive(Debug, Clone)]
pub struct FollowRedirect {
    pub new_url: Url,
    pub new_method: Method,
    pub new_body: Option<Bytes>,
    pub new_headers: HeaderMap,
}

/// Ação resultante da análise de uma resposta de redirecionamento.
#[derive(Debug)]
pub enum RedirectAction {
    /// O redirecionamento deve ser seguido para a nova URL.
    Follow(Box<FollowRedirect>),
    /// A resposta não é um redirecionamento ou não deve ser seguida.
    None,
}

/// Analisa a resposta HTTP para verificar se é um redirecionamento e calcula a nova requisição.
pub fn handle_redirect(
    current_req: &Request,
    status: StatusCode,
    response_headers: &HeaderMap,
    visited_urls: &mut HashSet<String>,
    max_redirects: usize,
) -> NetResult<RedirectAction> {
    // Apenas status 301, 302, 303, 307, 308 representam redirecionamentos HTTP padrão
    if !matches!(
        status,
        StatusCode::MOVED_PERMANENTLY
            | StatusCode::FOUND
            | StatusCode::SEE_OTHER
            | StatusCode::TEMPORARY_REDIRECT
            | StatusCode::PERMANENT_REDIRECT
    ) {
        return Ok(RedirectAction::None);
    }

    // Extrai o cabeçalho Location
    let location_raw = match response_headers.get(LOCATION).and_then(|v| v.to_str().ok()) {
        Some(loc) => loc.trim(),
        None => return Ok(RedirectAction::None), // Sem Location, trata como resposta normal
    };

    // Resolve URL relativa contra a URL atual
    let new_url = current_req.url.join(location_raw).map_err(|e| {
        NetError::InvalidUrl(format!("Location '{}' inválido: {}", location_raw, e))
    })?;

    // Verifica limite de saltos
    if visited_urls.len() >= max_redirects {
        return Err(NetError::TooManyRedirects(max_redirects));
    }

    // Detecção de loop (se a URL já foi visitada nesta cadeia de redirecionamentos)
    let url_key = new_url.to_string();
    if visited_urls.contains(&url_key) {
        return Err(NetError::TooManyRedirects(visited_urls.len()));
    }
    visited_urls.insert(url_key);

    // Determina o novo método e corpo conforme a semântica HTTP
    let (new_method, new_body) = match status {
        StatusCode::SEE_OTHER => (Method::GET, None),
        StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND => {
            if current_req.method == Method::POST {
                // Conversão histórica de POST para GET em 301 e 302
                (Method::GET, None)
            } else {
                (current_req.method.clone(), current_req.body.clone())
            }
        }
        StatusCode::TEMPORARY_REDIRECT | StatusCode::PERMANENT_REDIRECT => {
            // 307 e 308 preservam estritamente o método e o corpo
            (current_req.method.clone(), current_req.body.clone())
        }
        _ => (current_req.method.clone(), current_req.body.clone()),
    };

    // Higienização de cabeçalhos para proteção de privacidade
    let mut new_headers = current_req.headers.clone();

    // Se o método foi convertido para GET (ex: 301, 302, 303 de um POST),
    // remove cabeçalhos de representação de corpo para cumprir WHATWG Fetch e RFC 9110 §8.6.
    if new_method == Method::GET && current_req.method != Method::GET {
        new_headers.remove(CONTENT_TYPE);
        new_headers.remove(CONTENT_LENGTH);
        new_headers.remove(CONTENT_ENCODING);
    }

    let current_origin = Origin::parse(current_req.url.as_str()).unwrap_or_else(|_| Origin::new_opaque());
    let target_origin = Origin::parse(new_url.as_str()).unwrap_or_else(|_| Origin::new_opaque());

    // Se for redirecionamento cross-origin, remove cabeçalhos sensíveis
    if current_origin != target_origin {
        new_headers.remove(AUTHORIZATION);
        new_headers.remove(COOKIE);
        new_headers.remove("proxy-authorization");
    }

    Ok(RedirectAction::Follow(Box::new(FollowRedirect {
        new_url,
        new_method,
        new_body,
        new_headers,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;

    #[test]
    fn test_relative_redirect_resolution() {
        let req = Request::get("https://example.com/blog/post1").unwrap().build();
        let mut visited = HashSet::new();
        let mut resp_headers = HeaderMap::new();
        resp_headers.insert(LOCATION, HeaderValue::from_static("/about"));

        let action = handle_redirect(
            &req,
            StatusCode::FOUND,
            &resp_headers,
            &mut visited,
            20,
        )
        .unwrap();

        match action {
            RedirectAction::Follow(follow) => {
                assert_eq!(follow.new_url.as_str(), "https://example.com/about");
                assert_eq!(follow.new_method, Method::GET);
            }
            _ => panic!("Esperado Follow redirect"),
        }
    }

    #[test]
    fn test_cross_origin_redirect_strips_authorization() {
        let mut req_builder = Request::post("https://auth.example.com/login").unwrap();
        req_builder = req_builder.header(AUTHORIZATION, HeaderValue::from_static("Bearer token_secret"));
        req_builder = req_builder.header(COOKIE, HeaderValue::from_static("session=abc"));
        let req = req_builder.build();

        let mut resp_headers = HeaderMap::new();
        resp_headers.insert(LOCATION, HeaderValue::from_static("https://otherdomain.com/landing"));

        let mut visited = HashSet::new();
        let action = handle_redirect(
            &req,
            StatusCode::SEE_OTHER,
            &resp_headers,
            &mut visited,
            20,
        )
        .unwrap();

        match action {
            RedirectAction::Follow(follow) => {
                assert_eq!(follow.new_url.as_str(), "https://otherdomain.com/landing");
                assert_eq!(follow.new_method, Method::GET);
                assert!(!follow.new_headers.contains_key(AUTHORIZATION));
                assert!(!follow.new_headers.contains_key(COOKIE));
            }
            _ => panic!("Esperado Follow redirect"),
        }
    }

    #[test]
    fn test_redirect_loop_detection() {
        let req = Request::get("https://example.com/loop").unwrap().build();
        let mut visited = HashSet::new();
        visited.insert(req.url.to_string());

        let mut resp_headers = HeaderMap::new();
        resp_headers.insert(LOCATION, HeaderValue::from_static("https://example.com/loop"));

        let result = handle_redirect(
            &req,
            StatusCode::FOUND,
            &resp_headers,
            &mut visited,
            20,
        );

        assert!(matches!(result, Err(NetError::TooManyRedirects(_))));
    }

    #[test]
    fn test_post_to_get_redirect_strips_body_headers() {
        let mut req = Request::post("https://example.com/submit").unwrap().build();
        req.headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        req.headers.insert(CONTENT_LENGTH, HeaderValue::from_static("42"));
        req.headers.insert(CONTENT_ENCODING, HeaderValue::from_static("gzip"));

        let mut resp_headers = HeaderMap::new();
        resp_headers.insert(LOCATION, HeaderValue::from_static("/thank-you"));
        let mut visited = HashSet::new();

        let action = handle_redirect(
            &req,
            StatusCode::SEE_OTHER, // 303 converte sempre para GET
            &resp_headers,
            &mut visited,
            20,
        )
        .unwrap();

        match action {
            RedirectAction::Follow(follow) => {
                assert_eq!(follow.new_method, Method::GET);
                assert!(!follow.new_headers.contains_key(CONTENT_TYPE));
                assert!(!follow.new_headers.contains_key(CONTENT_LENGTH));
                assert!(!follow.new_headers.contains_key(CONTENT_ENCODING));
            }
            _ => panic!("Esperado Follow redirect"),
        }
    }
}
