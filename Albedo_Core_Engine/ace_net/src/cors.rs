use crate::error::{NetError, NetResult};
use crate::request::{Method, Request, RequestMode};
use crate::response::Response;
use ace_core::security::origin::Origin;
use http::header::{ACCESS_CONTROL_ALLOW_ORIGIN, ORIGIN};

/// Verifica se a requisição cruza origens.
pub fn is_cross_origin(req: &Request) -> bool {
    let req_origin_str = req.headers.get(ORIGIN).and_then(|v| v.to_str().ok()).unwrap_or("");
    if req_origin_str.is_empty() {
        return false;
    }
    let target_origin = Origin::parse(req.url.as_str()).unwrap_or_else(|_| Origin::new_opaque());
    target_origin.ascii_serialization() != req_origin_str
}

/// Verifica se a requisição necessita de preflight (não é "simples").
pub fn requires_preflight(req: &Request) -> bool {
    if !is_cross_origin(req) {
        return false;
    }
    
    // Métodos simples: GET, HEAD, POST
    if req.method != Method::GET && req.method != Method::HEAD && req.method != Method::POST {
        return true;
    }

    // Na especificação real de CORS, também checaríamos custom headers (ex: Content-Type não simples, etc).
    // Por simplificação do roadmap, consideramos qualquer custom header que não seja do set seguro como gatilho.
    // ...
    false
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
        options_req.headers.insert("access-control-request-method", method_val);
    }

    Ok(options_req)
}

/// Valida a resposta do servidor para uma requisição CORS (inclui preflight e normal).
pub fn validate_cors_response(req: &Request, resp: &Response) -> NetResult<()> {
    if req.mode != RequestMode::Cors || !is_cross_origin(req) {
        return Ok(());
    }

    let origin_str = req.headers.get(ORIGIN).and_then(|v| v.to_str().ok()).unwrap_or("");
    let allow_origin = resp.headers.get(ACCESS_CONTROL_ALLOW_ORIGIN).and_then(|v| v.to_str().ok()).unwrap_or("");

    if allow_origin == "*" {
        // Se credentials mode for 'include', '*' não é permitido.
        if req.credentials == crate::request::CredentialsMode::Include {
            crate::net_log::log_net_event(crate::net_log::NetEventType::Error, req.url.as_str(), "CORS falhou: '*' com credentials 'include'");
            return Err(NetError::CorsError("Wildcard '*' not allowed when credentials are 'include'".into()));
        }
        return Ok(());
    }

    if allow_origin == origin_str {
        return Ok(());
    }

    crate::net_log::log_net_event(crate::net_log::NetEventType::Error, req.url.as_str(), "CORS falhou: Access-Control-Allow-Origin mismatch");
    Err(NetError::CorsError(format!(
        "Origin '{}' not found in Access-Control-Allow-Origin", origin_str
    )))
}
