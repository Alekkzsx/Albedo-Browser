use crate::error::{NetError, NetResult};
use crate::http::request::{CredentialsMode, Method, Request, RequestMode};
use crate::http::response::Response;
use ace_core::security::origin::Origin;
use http::header::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_ORIGIN, ORIGIN, CONTENT_TYPE,
};

/// Verifica se a requisição cruza origens.
pub fn is_cross_origin(req: &Request) -> bool {
    let req_origin_str = req.headers.get(ORIGIN).and_then(|v| v.to_str().ok()).unwrap_or("");
    if req_origin_str.is_empty() {
        return false;
    }
    let target_origin = Origin::parse(req.url.as_str()).unwrap_or_else(|_| Origin::new_opaque());
    target_origin.ascii_serialization() != req_origin_str
}

/// Determina se um método HTTP é CORS-safelisted (GET, HEAD, POST).
#[inline]
pub fn is_safelisted_method(method: &Method) -> bool {
    *method == Method::GET || *method == Method::HEAD || *method == Method::POST
}

/// Determina se o valor de Content-Type é CORS-safelisted:
/// - application/x-www-form-urlencoded
/// - multipart/form-data
/// - text/plain
pub fn is_safelisted_content_type(ct: &str) -> bool {
    let essence = ct.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    matches!(
        essence.as_str(),
        "application/x-www-form-urlencoded" | "multipart/form-data" | "text/plain"
    )
}

/// Identifica se um cabeçalho é controlado pelo navegador / proibido para JS,
/// portanto não considerado um cabeçalho customizado que força preflight.
fn is_browser_managed_header(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    matches!(
        n.as_str(),
        "origin"
            | "referer"
            | "user-agent"
            | "host"
            | "connection"
            | "keep-alive"
            | "accept-encoding"
            | "content-length"
            | "priority"
            | "dnt"
            | "te"
            | "upgrade"
            | "upgrade-insecure-requests"
            | "range"
    ) || n.starts_with("sec-")
}

/// Verifica se um cabeçalho individual é não-safelisted segundo o W3C Fetch Standard §3.2.1.
fn is_non_safelisted_header(name: &http::HeaderName, val: &http::HeaderValue) -> bool {
    let name_str = name.as_str();

    // Cabeçalhos gerenciados pelo navegador não contam como custom headers
    if is_browser_managed_header(name_str) {
        return false;
    }

    let n = name_str.to_ascii_lowercase();
    match n.as_str() {
        "accept" | "accept-language" | "content-language" => false,
        "content-type" => {
            let val_str = val.to_str().unwrap_or("");
            !is_safelisted_content_type(val_str)
        }
        // Qualquer outro cabeçalho (ex: Authorization, X-Requested-With, X-Custom-*, etc.)
        _ => true,
    }
}

/// Coleta todos os nomes de cabeçalhos não-safelisted de uma requisição.
pub fn get_non_safelisted_headers(req: &Request) -> Vec<String> {
    let mut headers = Vec::new();
    for (name, val) in &req.headers {
        if is_non_safelisted_header(name, val) {
            headers.push(name.as_str().to_ascii_lowercase());
        }
    }
    headers.sort();
    headers.dedup();
    headers
}

/// Verifica se a requisição necessita de preflight OPTIONS conforme a W3C Fetch Spec §4.6.
pub fn requires_preflight(req: &Request) -> bool {
    if !is_cross_origin(req) {
        return false;
    }

    // 1. Método não é CORS-safelisted
    if !is_safelisted_method(&req.method) {
        return true;
    }

    // 2. Contém cabeçalhos não-safelisted (e.g. Content-Type: application/json, Authorization, etc.)
    !get_non_safelisted_headers(req).is_empty()
}

/// Constrói a requisição de Preflight OPTIONS baseada na requisição original.
pub fn build_preflight_request(req: &Request) -> NetResult<Request> {
    let mut options_req = Request::builder(req.url.clone(), Method::OPTIONS)?
        .mode(RequestMode::Cors)
        .build();

    if let Some(origin) = req.headers.get(ORIGIN) {
        options_req.headers.insert(ORIGIN, origin.clone());
    }

    // Injeta Access-Control-Request-Method
    if let Ok(method_val) = http::HeaderValue::from_str(req.method.as_str()) {
        options_req
            .headers
            .insert("access-control-request-method", method_val);
    }

    // Injeta Access-Control-Request-Headers caso haja headers customizados
    let non_safelisted = get_non_safelisted_headers(req);
    if !non_safelisted.is_empty() {
        let joined = non_safelisted.join(", ");
        if let Ok(hdr_val) = http::HeaderValue::from_str(&joined) {
            options_req
                .headers
                .insert("access-control-request-headers", hdr_val);
        }
    }

    Ok(options_req)
}

/// Valida a resposta do servidor para uma requisição CORS (inclui preflight e normal).
pub fn validate_cors_response(req: &Request, resp: &Response) -> NetResult<()> {
    if req.mode != RequestMode::Cors || !is_cross_origin(req) {
        return Ok(());
    }

    // Se for uma resposta a preflight OPTIONS, status deve ser de sucesso (200..=299)
    if req.method == Method::OPTIONS && !resp.status.is_success() {
        crate::telemetry::net_log::log_net_event(
            crate::telemetry::net_log::NetEventType::Error,
            req.url.as_str(),
            &format!("CORS preflight falhou com status {}", resp.status),
        );
        return Err(NetError::CorsError(format!(
            "CORS preflight failed with HTTP status {}",
            resp.status
        )));
    }

    let origin_str = req
        .headers
        .get(ORIGIN)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let allow_origin = resp
        .headers
        .get(ACCESS_CONTROL_ALLOW_ORIGIN)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if allow_origin.is_empty() {
        crate::telemetry::net_log::log_net_event(
            crate::telemetry::net_log::NetEventType::Error,
            req.url.as_str(),
            "CORS falhou: cabeçalho Access-Control-Allow-Origin ausente",
        );
        return Err(NetError::CorsError(
            "Access-Control-Allow-Origin header missing".into(),
        ));
    }

    if allow_origin == "*" {
        // Se credentials mode for 'include', '*' é estritamente proibido pela especificação
        if req.credentials == CredentialsMode::Include {
            crate::telemetry::net_log::log_net_event(
                crate::telemetry::net_log::NetEventType::Error,
                req.url.as_str(),
                "CORS falhou: '*' com credentials 'include'",
            );
            return Err(NetError::CorsError(
                "Wildcard '*' not allowed when credentials are 'include'".into(),
            ));
        }
        return Ok(());
    }

    if allow_origin == origin_str {
        // Se credentials mode for 'include', o servidor DEVE explicitar Access-Control-Allow-Credentials: true
        if req.credentials == CredentialsMode::Include {
            let allow_cred = resp
                .headers
                .get(ACCESS_CONTROL_ALLOW_CREDENTIALS)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if allow_cred != "true" {
                crate::telemetry::net_log::log_net_event(
                    crate::telemetry::net_log::NetEventType::Error,
                    req.url.as_str(),
                    "CORS falhou: Access-Control-Allow-Credentials != true",
                );
                return Err(NetError::CorsError(
                    "Credentials mode 'include' requires Access-Control-Allow-Credentials: true"
                        .into(),
                ));
            }
        }
        return Ok(());
    }

    crate::telemetry::net_log::log_net_event(
        crate::telemetry::net_log::NetEventType::Error,
        req.url.as_str(),
        "CORS falhou: Access-Control-Allow-Origin mismatch",
    );
    Err(NetError::CorsError(format!(
        "Origin '{}' not found in Access-Control-Allow-Origin (got '{}')",
        origin_str, allow_origin
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;
    use url::Url;

    #[test]
    fn test_cors_safelisted_methods() {
        assert!(is_safelisted_method(&Method::GET));
        assert!(is_safelisted_method(&Method::HEAD));
        assert!(is_safelisted_method(&Method::POST));
        assert!(!is_safelisted_method(&Method::PUT));
        assert!(!is_safelisted_method(&Method::DELETE));
        assert!(!is_safelisted_method(&Method::OPTIONS));
    }

    #[test]
    fn test_cors_safelisted_content_types() {
        assert!(is_safelisted_content_type("application/x-www-form-urlencoded"));
        assert!(is_safelisted_content_type("text/plain; charset=utf-8"));
        assert!(is_safelisted_content_type("multipart/form-data; boundary=something"));
        assert!(!is_safelisted_content_type("application/json"));
        assert!(!is_safelisted_content_type("application/xml"));
    }

    #[test]
    fn test_requires_preflight_on_custom_header() {
        let url = Url::parse("https://api.example.com/data").unwrap();
        let mut req = Request::builder(url, Method::POST).unwrap().build();
        req.headers.insert(ORIGIN, HeaderValue::from_static("https://app.com"));

        // Com Content-Type application/x-www-form-urlencoded -> simples
        req.headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        assert!(!requires_preflight(&req));

        // Mudando para application/json -> DEVE disparar preflight
        req.headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        assert!(requires_preflight(&req));

        // Adicionando Authorization -> DEVE disparar preflight
        let mut req2 = Request::builder(Url::parse("https://api.example.com/user").unwrap(), Method::GET)
            .unwrap()
            .build();
        req2.headers.insert(ORIGIN, HeaderValue::from_static("https://app.com"));
        req2.headers.insert("authorization", HeaderValue::from_static("Bearer token123"));
        assert!(requires_preflight(&req2));
    }

    #[test]
    fn test_build_preflight_request_includes_request_headers() {
        let url = Url::parse("https://api.example.com/data").unwrap();
        let mut req = Request::builder(url, Method::PUT).unwrap().build();
        req.headers.insert(ORIGIN, HeaderValue::from_static("https://app.com"));
        req.headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        req.headers.insert("x-custom-key", HeaderValue::from_static("val"));

        let preflight = build_preflight_request(&req).unwrap();
        assert_eq!(preflight.method, Method::OPTIONS);
        assert_eq!(
            preflight.headers.get("access-control-request-method").unwrap(),
            "PUT"
        );
        let req_headers = preflight
            .headers
            .get("access-control-request-headers")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(req_headers.contains("content-type"));
        assert!(req_headers.contains("x-custom-key"));
    }
}

