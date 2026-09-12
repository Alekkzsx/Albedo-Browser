//! # Entrada e Validações de Conformidade RFC 9111 (`CacheEntry`)
//!
//! Implementação formal do algoritmo de controle de frescor (Freshness Lifetime),
//! cálculo de idade (Age Calculation), heurística de validade e revalidação condicional.

use bytes::Bytes;
use http::header::{HeaderMap, CACHE_CONTROL, DATE, ETAG, EXPIRES, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use http::{Method, StatusCode};
use std::time::{Duration, SystemTime};
use url::Url;

/// Registro de um recurso armazenado no cache HTTP.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub url: Url,
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
    pub request_time: SystemTime,
    pub response_time: SystemTime,
}

impl CacheEntry {
    /// Cria uma nova entrada de cache a partir de uma resposta de rede.
    pub fn new(
        url: Url,
        status: StatusCode,
        headers: HeaderMap,
        body: Bytes,
        request_time: SystemTime,
        response_time: SystemTime,
    ) -> Self {
        Self {
            url,
            status,
            headers,
            body,
            request_time,
            response_time,
        }
    }

    /// Determina se a resposta pode ser armazenada em cache conforme RFC 9111 §3.
    pub fn is_cacheable(method: &Method, status: StatusCode, headers: &HeaderMap) -> bool {
        // Apenas métodos GET e HEAD são cacheáveis por padrão
        if method != Method::GET && method != Method::HEAD {
            return false;
        }

        // Verifica diretiva Cache-Control
        if let Some(cc) = headers.get(CACHE_CONTROL).and_then(|v| v.to_str().ok()) {
            let cc_lower = cc.to_ascii_lowercase();
            if cc_lower.contains("no-store") || cc_lower.contains("private") {
                return false;
            }
        }

        // Status codes cacheáveis por padrão (RFC 9111 §4.2.2)
        matches!(
            status,
            StatusCode::OK
                | StatusCode::NON_AUTHORITATIVE_INFORMATION
                | StatusCode::NO_CONTENT
                | StatusCode::PARTIAL_CONTENT
                | StatusCode::MULTIPLE_CHOICES
                | StatusCode::MOVED_PERMANENTLY
                | StatusCode::NOT_FOUND
                | StatusCode::METHOD_NOT_ALLOWED
                | StatusCode::GONE
                | StatusCode::URI_TOO_LONG
                | StatusCode::NOT_IMPLEMENTED
        )
    }

    /// Calcula o tempo de vida de frescor (Freshness Lifetime) da entrada (RFC 9111 §4.2.1).
    pub fn freshness_lifetime(&self) -> Duration {
        // 1. Diretiva max-age no Cache-Control
        if let Some(cc) = self.headers.get(CACHE_CONTROL).and_then(|v| v.to_str().ok()) {
            for directive in cc.split(',') {
                let part = directive.trim().to_ascii_lowercase();
                if let Some(stripped) = part.strip_prefix("max-age=") {
                    if let Ok(seconds) = stripped.trim().parse::<u64>() {
                        return Duration::from_secs(seconds);
                    }
                }
            }
        }

        // 2. Cabeçalho Expires menos Date
        if let (Some(expires), Some(date)) = (self.parse_http_date(EXPIRES), self.parse_http_date(DATE)) {
            if expires > date {
                if let Ok(diff) = expires.duration_since(date) {
                    return diff;
                }
            }
            return Duration::ZERO;
        }

        // 3. Heurística de frescor (RFC 9111 §4.2.2): 10% do intervalo (Date - Last-Modified)
        if let (Some(date), Some(last_modified)) = (self.parse_http_date(DATE), self.parse_http_date(LAST_MODIFIED)) {
            if date > last_modified {
                if let Ok(diff) = date.duration_since(last_modified) {
                    return diff / 10;
                }
            }
        }

        // Default seguro de 5 minutos caso não haja nenhuma indicação
        Duration::from_secs(300)
    }

    /// Calcula a idade atual da resposta (RFC 9111 §4.2.3).
    pub fn current_age(&self, now: SystemTime) -> Duration {
        let date_value = self.parse_http_date(DATE).unwrap_or(self.response_time);

        // apparent_age = max(0, response_time - date_value)
        let apparent_age = self
            .response_time
            .duration_since(date_value)
            .unwrap_or(Duration::ZERO);

        // response_delay = response_time - request_time
        let response_delay = self
            .response_time
            .duration_since(self.request_time)
            .unwrap_or(Duration::ZERO);

        // corrected_initial_age = max(apparent_age, corrected_age_value)
        let corrected_initial_age = apparent_age + response_delay;

        // resident_time = now - response_time
        let resident_time = now.duration_since(self.response_time).unwrap_or(Duration::ZERO);

        corrected_initial_age + resident_time
    }

    /// Determina se a entrada ainda está fresca no instante `now`.
    pub fn is_fresh(&self, now: SystemTime) -> bool {
        // Se houver no-cache explícito, exige sempre revalidação
        if let Some(cc) = self.headers.get(CACHE_CONTROL).and_then(|v| v.to_str().ok()) {
            if cc.to_ascii_lowercase().contains("no-cache") {
                return false;
            }
        }

        self.current_age(now) < self.freshness_lifetime()
    }

    /// Gera os cabeçalhos de revalidação condicional (`If-None-Match` e `If-Modified-Since`).
    pub fn conditional_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        if let Some(etag) = self.headers.get(ETAG) {
            headers.insert(IF_NONE_MATCH, etag.clone());
        }

        if let Some(last_mod) = self.headers.get(LAST_MODIFIED) {
            headers.insert(IF_MODIFIED_SINCE, last_mod.clone());
        }

        headers
    }

    /// Atualiza os cabeçalhos desta entrada a partir de uma resposta `304 Not Modified` (RFC 9111 §4.3.4).
    pub fn update_from_304(&mut self, new_headers: &HeaderMap, response_time: SystemTime) {
        self.response_time = response_time;
        for (key, val) in new_headers {
            // Atualiza cabeçalhos que controlam cache e validação
            if key == CACHE_CONTROL || key == EXPIRES || key == DATE || key == ETAG {
                self.headers.insert(key.clone(), val.clone());
            }
        }
    }

    /// Auxiliar de parsing de datas no formato HTTP-date (RFC 9110 §5.6.7).
    fn parse_http_date(&self, header_name: http::header::HeaderName) -> Option<SystemTime> {
        let header_str = self.headers.get(header_name)?.to_str().ok()?;
        httpdate::parse_http_date(header_str).ok()
    }

    /// Serializa a entrada de cache para formato binário compacto.
    pub fn to_bytes(&self) -> Bytes {
        use bytes::BufMut;
        let mut buf = bytes::BytesMut::new();
        // url length & string
        let url_bytes = self.url.as_str().as_bytes();
        buf.put_u32_le(url_bytes.len() as u32);
        buf.put_slice(url_bytes);
        // status code
        buf.put_u16_le(self.status.as_u16());
        // headers
        buf.put_u32_le(self.headers.len() as u32);
        for (name, value) in &self.headers {
            let name_bytes = name.as_str().as_bytes();
            buf.put_u16_le(name_bytes.len() as u16);
            buf.put_slice(name_bytes);
            let value_bytes = value.as_bytes();
            buf.put_u32_le(value_bytes.len() as u32);
            buf.put_slice(value_bytes);
        }
        // body
        buf.put_u32_le(self.body.len() as u32);
        buf.put_slice(&self.body);
        // request_time
        let req_duration = self.request_time.duration_since(std::time::UNIX_EPOCH).unwrap_or(Duration::ZERO);
        buf.put_u64_le(req_duration.as_secs());
        buf.put_u32_le(req_duration.subsec_nanos());
        // response_time
        let res_duration = self.response_time.duration_since(std::time::UNIX_EPOCH).unwrap_or(Duration::ZERO);
        buf.put_u64_le(res_duration.as_secs());
        buf.put_u32_le(res_duration.subsec_nanos());
        
        buf.freeze()
    }

    /// Desserializa a entrada de cache a partir de um buffer binário.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        use bytes::Buf;
        let mut buf = data;
        if buf.remaining() < 4 { return None; }
        let url_len = buf.get_u32_le() as usize;
        if buf.remaining() < url_len { return None; }
        let url_str = std::str::from_utf8(&buf[..url_len]).ok()?;
        let url = Url::parse(url_str).ok()?;
        buf.advance(url_len);
        
        if buf.remaining() < 2 { return None; }
        let status = StatusCode::from_u16(buf.get_u16_le()).ok()?;
        
        if buf.remaining() < 4 { return None; }
        let num_headers = buf.get_u32_le() as usize;
        let mut headers = HeaderMap::with_capacity(num_headers);
        for _ in 0..num_headers {
            if buf.remaining() < 2 { return None; }
            let name_len = buf.get_u16_le() as usize;
            if buf.remaining() < name_len { return None; }
            let name_str = std::str::from_utf8(&buf[..name_len]).ok()?;
            let header_name = http::header::HeaderName::try_from(name_str).ok()?;
            buf.advance(name_len);
            
            if buf.remaining() < 4 { return None; }
            let val_len = buf.get_u32_le() as usize;
            if buf.remaining() < val_len { return None; }
            let header_val = http::header::HeaderValue::from_bytes(&buf[..val_len]).ok()?;
            buf.advance(val_len);
            
            headers.insert(header_name, header_val);
        }
        
        if buf.remaining() < 4 { return None; }
        let body_len = buf.get_u32_le() as usize;
        if buf.remaining() < body_len { return None; }
        let body = Bytes::copy_from_slice(&buf[..body_len]);
        buf.advance(body_len);
        
        if buf.remaining() < 12 { return None; }
        let req_secs = buf.get_u64_le();
        let req_nanos = buf.get_u32_le();
        let request_time = std::time::UNIX_EPOCH + Duration::new(req_secs, req_nanos);
        
        if buf.remaining() < 12 { return None; }
        let res_secs = buf.get_u64_le();
        let res_nanos = buf.get_u32_le();
        let response_time = std::time::UNIX_EPOCH + Duration::new(res_secs, res_nanos);
        
        Some(Self {
            url,
            status,
            headers,
            body,
            request_time,
            response_time,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;

    #[test]
    fn test_cache_control_max_age_freshness() {
        let mut headers = HeaderMap::new();
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("public, max-age=3600"));

        let now = SystemTime::now();
        let entry = CacheEntry::new(
            Url::parse("https://example.com/asset.js").unwrap(),
            StatusCode::OK,
            headers,
            Bytes::from_static(b"console.log('hi');"),
            now,
            now,
        );

        assert_eq!(entry.freshness_lifetime(), Duration::from_secs(3600));
        assert!(entry.is_fresh(now));

        // 3601 segundos depois deve estar stale
        let later = now + Duration::from_secs(3601);
        assert!(!entry.is_fresh(later));
    }

    #[test]
    fn test_conditional_headers_generation() {
        let mut headers = HeaderMap::new();
        headers.insert(ETAG, HeaderValue::from_static("\"xyz123\""));
        headers.insert(LAST_MODIFIED, HeaderValue::from_static("Wed, 21 Oct 2025 07:28:00 GMT"));

        let now = SystemTime::now();
        let entry = CacheEntry::new(
            Url::parse("https://example.com/data.json").unwrap(),
            StatusCode::OK,
            headers,
            Bytes::from_static(b"{}"),
            now,
            now,
        );

        let cond = entry.conditional_headers();
        assert_eq!(cond.get(IF_NONE_MATCH).unwrap(), "\"xyz123\"");
        assert_eq!(cond.get(IF_MODIFIED_SINCE).unwrap(), "Wed, 21 Oct 2025 07:28:00 GMT");
    }
}
