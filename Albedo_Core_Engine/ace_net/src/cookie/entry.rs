//! # Modelagem de Cookies HTTP (RFC 6265bis + CHIPS Partitioned)
//!
//! Implementa os atributos e o parser normativo de cookies HTTP, suportando
//! `SameSite=Lax` por padrão, expiração via `Max-Age` / `Expires`, restrições
//! de segurança `Secure` / `HttpOnly` e isolamento de estado particionado (**CHIPS**).

use crate::request::CredentialsMode;
use std::time::{Duration, SystemTime};
use url::Url;

/// Política de envio de cookies entre origens distintas (RFC 6265bis §5.3.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SameSite {
    /// O cookie é enviado apenas em requisições estritamente originadas do mesmo site.
    Strict,
    /// (Padrão moderno) O cookie é enviado em navegações top-level seguras (GET), mas omitido em sub-recursos cross-site.
    #[default]
    Lax,
    /// O cookie é enviado em qualquer contexto cross-site (exige o atributo `Secure`).
    None,
}

impl SameSite {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "Strict",
            Self::Lax => "Lax",
            Self::None => "None",
        }
    }
}

/// Representação estruturada de um Cookie HTTP em conformidade com RFC 6265bis e CHIPS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires_at: Option<SystemTime>,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSite,
    /// Chave de particionamento (CHIPS) derivada do site de nível superior.
    pub partition_key: Option<String>,
}

impl Cookie {
    /// Verifica se o cookie ainda não expirou.
    pub fn is_fresh(&self, now: SystemTime) -> bool {
        match self.expires_at {
            Some(exp) => exp > now,
            None => true, // Cookie de sessão vive até o fechamento
        }
    }

    /// Analisa o valor de um cabeçalho `Set-Cookie`.
    pub fn parse(
        header_str: &str,
        request_url: &Url,
        top_level_site: Option<&str>,
        now: SystemTime,
    ) -> Option<Self> {
        let mut parts = header_str.split(';');
        let name_value = parts.next()?.trim();

        let eq_idx = name_value.find('=')?;
        let name = name_value[..eq_idx].trim().to_string();
        let value = name_value[eq_idx + 1..].trim().to_string();

        if name.is_empty() {
            return None;
        }

        let default_domain = request_url.host_str().unwrap_or("").to_ascii_lowercase();
        let mut domain = default_domain;
        let mut path = "/".to_string();
        let mut expires_at = None;
        let mut secure = false;
        let mut http_only = false;
        let mut same_site = SameSite::Lax;
        let mut is_partitioned = false;

        for part in parts {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            let mut attr_parts = part.splitn(2, '=');
            let attr_name = attr_parts.next()?.trim().to_ascii_lowercase();
            let attr_val = attr_parts.next().map(|v| v.trim());

            match attr_name.as_str() {
                "secure" => secure = true,
                "httponly" => http_only = true,
                "partitioned" => is_partitioned = true,
                "domain" => {
                    if let Some(d) = attr_val {
                        let clean = d.trim_start_matches('.').to_ascii_lowercase();
                        if !clean.is_empty() {
                            domain = clean;
                        }
                    }
                }
                "path" => {
                    if let Some(p) = attr_val {
                        if p.starts_with('/') {
                            path = p.to_string();
                        }
                    }
                }
                "max-age" => {
                    if let Some(ma_str) = attr_val {
                        if let Ok(delta_secs) = ma_str.parse::<i64>() {
                            if delta_secs <= 0 {
                                expires_at = Some(SystemTime::UNIX_EPOCH);
                            } else {
                                expires_at = Some(now + Duration::from_secs(delta_secs as u64));
                            }
                        }
                    }
                }
                "expires" => {
                    if expires_at.is_none() {
                        if let Some(date_str) = attr_val {
                            if let Ok(t) = httpdate::parse_http_date(date_str) {
                                expires_at = Some(t);
                            }
                        }
                    }
                }
                "samesite" => {
                    if let Some(ss) = attr_val {
                        if ss.eq_ignore_ascii_case("strict") {
                            same_site = SameSite::Strict;
                        } else if ss.eq_ignore_ascii_case("none") {
                            same_site = SameSite::None;
                        } else {
                            same_site = SameSite::Lax;
                        }
                    }
                }
                _ => {}
            }
        }

        // CHIPS: Cookies com SameSite=None ou Partitioned devem ter o atributo Secure
        let partition_key = if is_partitioned && secure {
            top_level_site.map(|s| s.to_string())
        } else {
            None
        };

        Some(Self {
            name,
            value,
            domain,
            path,
            expires_at,
            secure,
            http_only,
            same_site,
            partition_key,
        })
    }

    /// Avalia se este cookie pode ser enviado para a requisição de destino.
    pub fn is_valid_for_request(
        &self,
        request_url: &Url,
        top_level_site: Option<&str>,
        credentials_mode: CredentialsMode,
        is_same_site: bool,
        is_navigation_get: bool,
        now: SystemTime,
    ) -> bool {
        // 1. Não expirado
        if !self.is_fresh(now) {
            return false;
        }

        // 2. Modo de credenciais
        if credentials_mode == CredentialsMode::Omit {
            return false;
        }

        if credentials_mode == CredentialsMode::SameOrigin && !is_same_site {
            return false;
        }

        // 3. Validação de protocolo seguro (Secure flag exige HTTPS)
        if self.secure && request_url.scheme() != "https" {
            return false;
        }

        // 4. Validação de Domínio
        let req_host = match request_url.host_str() {
            Some(h) => h.to_ascii_lowercase(),
            None => return false,
        };

        if req_host != self.domain && !req_host.ends_with(&format!(".{}", self.domain)) {
            return false;
        }

        // 5. Validação de Path
        let req_path = request_url.path();
        if !req_path.starts_with(&self.path) {
            return false;
        }

        // 6. Política de SameSite
        match self.same_site {
            SameSite::Strict => {
                if !is_same_site {
                    return false;
                }
            }
            SameSite::Lax => {
                // Lax é enviado em cross-site apenas se for navegação top-level segura (GET)
                if !is_same_site && !is_navigation_get {
                    return false;
                }
            }
            SameSite::None => {
                // Exige Secure para cross-site
                if !self.secure {
                    return false;
                }
            }
        }

        // 7. CHIPS Partition Key Matching
        if let Some(ref cookie_part) = self.partition_key {
            match top_level_site {
                Some(req_part) if req_part == cookie_part => {}
                _ => return false,
            }
        }

        true
    }
}
