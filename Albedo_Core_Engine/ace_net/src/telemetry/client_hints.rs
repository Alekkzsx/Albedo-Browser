//! # HTTP Client Hints (RFC 8942) & Modernização de Metadados de Cliente
//!
//! Fornece aos servidores cabeçalhos estruturados com metadados do navegador e plataforma,
//! reduzindo a necessidade de parsing de User-Agent strings complexas:
//! - `Sec-CH-UA`: Lista de marcas e versões do motor
//! - `Sec-CH-UA-Mobile`: Indicador booleano (`?0` para desktop, `?1` para mobile)
//! - `Sec-CH-UA-Platform`: Nome do sistema operacional

use http::header::HeaderMap;
use http::{HeaderName, HeaderValue};

/// Versão padrão e marcas de compatibilidade do Albedo Browser.
pub const DEFAULT_SEC_CH_UA: &str = "\"Albedo\";v=\"0.1\", \"Chromium\";v=\"130\", \"Not?A_Brand\";v=\"99\"";
pub const DEFAULT_SEC_CH_UA_PLATFORM: &str = "\"Windows\"";

/// Injeta automaticamente os cabeçalhos essenciais de Client Hints modernos na requisição.
pub fn inject_default_client_hints(headers: &mut HeaderMap) {
    let ua_name = HeaderName::from_static("sec-ch-ua");
    let mobile_name = HeaderName::from_static("sec-ch-ua-mobile");
    let platform_name = HeaderName::from_static("sec-ch-ua-platform");

    if !headers.contains_key(&ua_name) {
        headers.insert(ua_name, HeaderValue::from_static(DEFAULT_SEC_CH_UA));
    }

    if !headers.contains_key(&mobile_name) {
        headers.insert(mobile_name, HeaderValue::from_static("?0"));
    }

    if !headers.contains_key(&platform_name) {
        headers.insert(platform_name, HeaderValue::from_static(DEFAULT_SEC_CH_UA_PLATFORM));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_default_client_hints() {
        let mut headers = HeaderMap::new();
        inject_default_client_hints(&mut headers);

        assert_eq!(headers.get("sec-ch-ua").unwrap(), DEFAULT_SEC_CH_UA);
        assert_eq!(headers.get("sec-ch-ua-mobile").unwrap(), "?0");
        assert_eq!(headers.get("sec-ch-ua-platform").unwrap(), DEFAULT_SEC_CH_UA_PLATFORM);
    }
}
