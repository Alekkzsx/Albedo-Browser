use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Origin {
    pub scheme: String,
    pub host: String,
    pub port: Option<u16>,
}

impl Origin {
    pub fn from_url(url: &str) -> Option<Self> {
        let parsed = crate::ace::url::parse(url, None).ok()?;
        Some(Self {
            scheme: parsed.scheme().to_string(),
            host: parsed.host_str()?.to_string(),
            port: parsed.port_or_known_default(),
        })
    }

    pub fn is_same_origin(&self, other: &Origin) -> bool {
        self.scheme == other.scheme && self.host == other.host && self.port == other.port
    }

    pub fn is_secure(&self) -> bool {
        self.scheme == "https" || self.scheme == "albedo"
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(port) = self.port {
            write!(f, "{}://{}:{}", self.scheme, self.host, port)
        } else {
            write!(f, "{}://{}", self.scheme, self.host)
        }
    }
}

pub struct SecurityContext {
    pub current_origin: Origin,
    pub csp: Option<ContentSecurityPolicy>,
}

pub struct ContentSecurityPolicy {
    pub raw: String,
    // Add specific directives as needed
    pub script_src: Vec<String>,
    pub connect_src: Vec<String>,
}

impl ContentSecurityPolicy {
    pub fn parse(raw: &str) -> Self {
        let mut script_src = Vec::new();
        let mut connect_src = Vec::new();

        for directive in raw.split(';') {
            let parts: Vec<&str> = directive.trim().split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            match parts[0] {
                "script-src" => script_src.extend(parts[1..].iter().map(|s| s.to_string())),
                "connect-src" => connect_src.extend(parts[1..].iter().map(|s| s.to_string())),
                _ => {}
            }
        }

        Self {
            raw: raw.to_string(),
            script_src,
            connect_src,
        }
    }

    pub fn allows_connect(&self, url: &str, origin: &Origin) -> bool {
        self.check_directive(&self.connect_src, url, origin)
    }

    pub fn allows_script(&self, url: &str, origin: &Origin) -> bool {
        self.check_directive(&self.script_src, url, origin)
    }

    fn check_directive(&self, directive: &[String], url: &str, origin: &Origin) -> bool {
        if directive.is_empty() {
            return true;
        }
        if directive.contains(&"'none'".to_string()) {
            return false;
        }
        if directive.contains(&"*".to_string()) {
            return true;
        }

        if directive.contains(&"'self'".to_string()) {
            if let Some(target_origin) = Origin::from_url(url) {
                if origin.is_same_origin(&target_origin) {
                    return true;
                }
            }
        }

        // Check for specific hosts
        for dev in directive {
            if url.contains(dev) {
                return true;
            }
        }

        false
    }
}

pub struct AccessControl {
    pub allowed_origins: Vec<String>,
}

impl AccessControl {
    pub fn new() -> Self {
        Self {
            allowed_origins: Vec::new(),
        }
    }

    pub fn validate_cors(
        &self,
        origin: &Origin,
        _target_url: &str,
        resp_headers: &std::collections::HashMap<String, String>,
    ) -> bool {
        let allow_origin = resp_headers
            .get("access-control-allow-origin")
            .map(|s| s.as_str());
        let allow_credentials = resp_headers
            .get("access-control-allow-credentials")
            .map(|s| s.as_str())
            == Some("true");

        if let Some(mut origin_val) = allow_origin {
            origin_val = origin_val.trim();
            if allow_credentials && origin_val == "*" {
                // Segurança W3C: Se Credentials=true, wildcard '*' não é permitido no Allow-Origin
                return false;
            }
            if origin_val == "*" {
                return true;
            }
            if origin_val == origin.to_string().as_str() {
                return true;
            }
        }
        false
    }
}

pub struct CookieJar {
    cookies: std::collections::HashMap<String, Vec<Cookie>>,
}

#[derive(Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: String, // Strict, Lax, None
}

impl CookieJar {
    pub fn new() -> Self {
        Self {
            cookies: std::collections::HashMap::new(),
        }
    }

    pub fn set_cookie(&mut self, url: &str, cookie_str: &str) {
        // Very simplified cookie parser
        let origin = Origin::from_url(url);
        if let Some(org) = origin {
            let parts: Vec<&str> = cookie_str.split(';').collect();
            if parts.is_empty() {
                return;
            }
            let name_val: Vec<&str> = parts[0].splitn(2, '=').collect();
            if name_val.len() < 2 {
                return;
            }

            let cookie = Cookie {
                name: name_val[0].trim().to_string(),
                value: name_val[1].trim().to_string(),
                domain: org.host.clone(),
                path: "/".to_string(),
                secure: cookie_str.contains("Secure"),
                http_only: cookie_str.contains("HttpOnly"),
                same_site: "Lax".to_string(),
            };

            self.cookies.entry(org.host).or_default().push(cookie);
        }
    }

    pub fn get_cookies_for_url(&self, url: &str) -> String {
        let origin = Origin::from_url(url);
        if let Some(org) = origin {
            if let Some(cookies) = self.cookies.get(&org.host) {
                return cookies
                    .iter()
                    .map(|c| format!("{}={}", c.name, c.value))
                    .collect::<Vec<_>>()
                    .join("; ");
            }
        }
        String::new()
    }
}
