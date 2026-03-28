use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlError {
    InvalidScheme,
    InvalidHost,
    InvalidPort,
    InvalidIPv4,
    InvalidIPv6,
    MissingScheme,
    RelativeUrlWithoutBase,
    ParseError(&'static str),
}

impl fmt::Display for UrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UrlError::InvalidScheme => write!(f, "Invalid scheme"),
            UrlError::InvalidHost => write!(f, "Invalid host"),
            UrlError::InvalidPort => write!(f, "Invalid port"),
            UrlError::InvalidIPv4 => write!(f, "Invalid IPv4 address"),
            UrlError::InvalidIPv6 => write!(f, "Invalid IPv6 address"),
            UrlError::MissingScheme => write!(f, "Missing scheme"),
            UrlError::RelativeUrlWithoutBase => write!(f, "Relative URL without base URL"),
            UrlError::ParseError(m) => write!(f, "Parse error: {}", m),
        }
    }
}

impl std::error::Error for UrlError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Host {
    Domain(String),
    Ipv4(std::net::Ipv4Addr),
    Ipv6(std::net::Ipv6Addr),
    Empty,
}

impl std::fmt::Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Host::Domain(s) => write!(f, "{}", s),
            Host::Ipv4(addr) => write!(f, "{}", addr),
            Host::Ipv6(addr) => write!(f, "[{}]", addr),
            Host::Empty => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    pub scheme: String,
    pub username: String,
    pub password: Option<String>,
    pub host: Option<Host>,
    pub port: Option<u16>,
    pub path: Vec<String>,
    pub query: Option<String>,
    pub fragment: Option<String>,
}

impl Url {
    pub fn parse(input: &str, base: Option<&Url>) -> std::result::Result<Url, UrlError> {
        super::parser::parse(input, base)
    }

    pub fn is_special(&self) -> bool {
        matches!(
            self.scheme.as_str(),
            "http" | "https" | "ws" | "wss" | "ftp" | "file"
        )
    }

    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    pub fn host_str(&self) -> Option<String> {
        self.host.as_ref().map(|h| h.to_string())
    }

    pub fn as_str(&self) -> String {
        self.to_string()
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn path(&self) -> String {
        if self.path.is_empty() {
            return "/".to_string();
        }
        format!("/{}", self.path.join("/"))
    }

    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_deref()
    }

    pub fn origin(&self) -> String {
        if !self.is_special() {
            return "null".to_string();
        }
        let host_display = self
            .host
            .as_ref()
            .map(|h| h.to_string())
            .unwrap_or_default();
        let mut origin = format!("{}://{}", self.scheme, host_display);
        if let Some(p) = self.port {
            if Some(p) != self.default_port() {
                origin.push_str(&format!(":{}", p));
            }
        }
        origin
    }

    pub fn port_or_known_default(&self) -> Option<u16> {
        self.port.or_else(|| self.default_port())
    }

    pub fn join(&self, input: &str) -> std::result::Result<Url, UrlError> {
        // Resolução relativa direta para casos comuns de navegação.
        // Mantém parser como fallback para entradas completas/complexas.
        if input.contains("://") {
            return super::parser::parse(input, None);
        }

        let mut out = self.clone();
        out.query = None;
        out.fragment = None;

        if input.starts_with('/') {
            out.path.clear();
            for seg in input.trim_start_matches('/').split('/') {
                if seg.is_empty() || seg == "." {
                    continue;
                }
                if seg == ".." {
                    out.path.pop();
                } else {
                    out.path.push(seg.to_string());
                }
            }
            return Ok(out);
        }

        let mut raw = input;
        if let Some(hash_pos) = raw.find('#') {
            out.fragment = Some(raw[hash_pos + 1..].to_string());
            raw = &raw[..hash_pos];
        }
        if let Some(q_pos) = raw.find('?') {
            out.query = Some(raw[q_pos + 1..].to_string());
            raw = &raw[..q_pos];
        }

        if !out.path.is_empty() {
            out.path.pop();
        }
        for seg in raw.split('/') {
            if seg.is_empty() || seg == "." {
                continue;
            }
            if seg == ".." {
                out.path.pop();
            } else {
                out.path.push(seg.to_string());
            }
        }

        Ok(out)
    }

    pub fn default_port(&self) -> Option<u16> {
        match self.scheme.as_str() {
            "http" | "ws" => Some(80),
            "https" | "wss" => Some(443),
            "ftp" => Some(21),
            _ => None,
        }
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:", self.scheme)?;
        if let Some(ref host) = self.host {
            write!(f, "//")?;
            if !self.username.is_empty() {
                write!(f, "{}", self.username)?;
                if let Some(ref pass) = self.password {
                    write!(f, ":{}", pass)?;
                }
                write!(f, "@")?;
            }
            match host {
                Host::Domain(ref d) => write!(f, "{}", d)?,
                Host::Ipv4(ref addr) => write!(f, "{}", addr)?,
                Host::Ipv6(ref addr) => write!(f, "[{}]", addr)?,
                Host::Empty => {}
            }
            if let Some(port) = self.port {
                if Some(port) != self.default_port() {
                    write!(f, ":{}", port)?;
                }
            }
        } else if self.scheme == "file" {
            write!(f, "//")?;
        }

        if !self.path.is_empty() {
            for segment in &self.path {
                write!(f, "/{}", segment)?;
            }
        } else if self.host.is_some() || self.scheme == "file" {
            write!(f, "/")?;
        }

        if let Some(ref query) = self.query {
            f.write_str("?")?;
            f.write_str(query)?;
        }

        if let Some(ref fragment) = self.fragment {
            f.write_str("#")?;
            f.write_str(fragment)?;
        }

        Ok(())
    }
}
