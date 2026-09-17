//! # Modelagem de Cookies HTTP (RFC 6265bis + CHIPS Partitioned)
//!
//! Implementa os atributos e o parser normativo de cookies HTTP, suportando
//! `SameSite=Lax` por padrão, expiração via `Max-Age` / `Expires`, restrições
//! de segurança `Secure` / `HttpOnly` e isolamento de estado particionado (**CHIPS**).

use crate::http::request::CredentialsMode;
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



/// Verifica se uma cadeia de domínio representa um sufixo público registrado.
pub fn is_public_suffix(domain: &str) -> bool {
    let clean = domain.trim_start_matches('.').to_ascii_lowercase();
    if !clean.contains('.') {
        // TLDs de primeiro nível como "com", "org", "io" sempre são sufixos públicos
        return true;
    }

    use psl::Psl;
    psl::List.domain(clean.as_bytes()).is_none()
}

/// Valida se o atributo Domain é seguro para o host requisitante (RFC 6265bis §5.4):
/// 1. Não pode ser um sufixo público (ex: "com" ou "co.uk").
/// 2. O host requisitante deve ser idêntico ou terminar com "." + cookie_domain (domain-matching).
pub fn is_valid_cookie_domain(cookie_domain: &str, request_host: &str) -> bool {
    let clean_cookie = cookie_domain.trim_start_matches('.').to_ascii_lowercase();
    let clean_req = request_host.trim_start_matches('.').to_ascii_lowercase();

    if is_public_suffix(&clean_cookie) {
        return false;
    }

    if clean_req == clean_cookie {
        return true;
    }

    if clean_req.ends_with(&format!(".{}", clean_cookie)) {
        return true;
    }

    false
}

impl Cookie {
    /// Verifica se o cookie ainda não expirou.
    pub fn is_fresh(&self, now: SystemTime) -> bool {
        match self.expires_at {
            Some(exp) => exp > now,
            None => true, // Cookie de sessão vive até o fechamento
        }
    }

    /// Serializa o cookie em uma linha de texto compacta para persistência em disco.
    /// Retorna `None` se o cookie for efêmero de sessão (sem `expires_at`).
    pub fn to_persistent_line(&self) -> Option<String> {
        let exp = self.expires_at?;
        let epoch_secs = exp.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
        let samesite_str = self.same_site.as_str();
        let part_key = self.partition_key.as_deref().unwrap_or("");
        Some(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            self.name,
            self.value,
            self.domain,
            self.path,
            epoch_secs,
            self.secure,
            self.http_only,
            samesite_str,
            part_key
        ))
    }

    /// Desserializa um cookie persistente a partir de uma linha de texto.
    pub fn from_persistent_line(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
        if parts.len() < 9 {
            return None;
        }
        let epoch_secs: u64 = parts[4].parse().ok()?;
        let expires_at = Some(std::time::UNIX_EPOCH + Duration::from_secs(epoch_secs));
        let secure = parts[5].parse().unwrap_or(false);
        let http_only = parts[6].parse().unwrap_or(false);
        let same_site = match parts[7] {
            "Strict" => SameSite::Strict,
            "None" => SameSite::None,
            _ => SameSite::Lax,
        };
        let partition_key = if parts[8].is_empty() {
            None
        } else {
            Some(parts[8].to_string())
        };

        Some(Self {
            name: parts[0].to_string(),
            value: parts[1].to_string(),
            domain: parts[2].to_string(),
            path: parts[3].to_string(),
            expires_at,
            secure,
            http_only,
            same_site,
            partition_key,
        })
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

        // RFC 6265bis: Limite normativo máximo de tamanho por cookie (4096 bytes)
        if name.len() + value.len() > 4096 {
            return None;
        }

        let default_domain = request_url.host_str().unwrap_or("").to_ascii_lowercase();
        let mut domain = default_domain.clone();
        let mut custom_domain = None;
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
                            custom_domain = Some(clean);
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

        // Validação de segurança de domínio (RFC 6265bis §5.4):
        // Rejeita a diretiva Domain se for um sufixo público ou não bater com o host de requisição
        if let Some(d) = custom_domain {
            if is_valid_cookie_domain(&d, &default_domain) {
                domain = d;
            } else {
                return None;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_suffix_detection() {
        assert!(is_public_suffix("com"));
        assert!(is_public_suffix("org"));
        assert!(is_public_suffix("co.uk"));
        assert!(is_public_suffix("com.br"));
        assert!(is_public_suffix("github.io"));

        assert!(!is_public_suffix("example.com"));
        assert!(!is_public_suffix("albedo.org"));
        assert!(!is_public_suffix("myproject.github.io"));
    }

    #[test]
    fn test_super_cookie_rejection() {
        let url = Url::parse("https://evil.example.co.uk/page").unwrap();
        let now = SystemTime::now();

        // Tentativa de injetar super-cookie para .co.uk
        let super_cookie = Cookie::parse("session=hacked; Domain=co.uk", &url, None, now);
        assert!(super_cookie.is_none(), "Super-cookie para .co.uk deve ser rejeitado");

        // Tentativa de injetar super-cookie para .com
        let url_com = Url::parse("https://evil.com/page").unwrap();
        let super_com = Cookie::parse("session=hacked; Domain=com", &url_com, None, now);
        assert!(super_com.is_none(), "Super-cookie para .com deve ser rejeitado");

        // Tentativa de injetar domínio fora do escopo (ex: evil.com tentando definir para other.com)
        let mismatch = Cookie::parse("session=hacked; Domain=other.com", &url_com, None, now);
        assert!(mismatch.is_none(), "Domínio divergente deve ser rejeitado");

        // Domínio válido para subdomínio
        let valid = Cookie::parse("session=ok; Domain=example.co.uk", &url, None, now);
        assert!(valid.is_some(), "Domínio de escopo correto deve ser aceito");
        assert_eq!(valid.unwrap().domain, "example.co.uk");
    }

    #[test]
    fn test_max_cookie_size() {
        let url = Url::parse("https://example.com").unwrap();
        let now = SystemTime::now();

        // Cookie gigante de 5000 bytes (excede limite de 4096)
        let huge_val = "x".repeat(5000);
        let header = format!("token={}", huge_val);
        let parsed = Cookie::parse(&header, &url, None, now);
        assert!(parsed.is_none(), "Cookie maior que 4096 bytes deve ser rejeitado");
    }
}

