use std::fmt;

#[derive(Clone, Debug)]
pub struct Url {
    pub href: String,
    pub origin: String,
    pub protocol: String,
    pub host: String,
    pub hostname: String,
    pub port: String,
    pub pathname: String,
    pub search: String,
    pub hash: String,
}

impl Url {
    pub fn parse(s: &str, _base: Option<&Url>) -> Result<Url, String> {
        Ok(Url {
            href: s.to_string(),
            origin: String::new(),
            protocol: String::new(),
            host: String::new(),
            hostname: String::new(),
            port: String::new(),
            pathname: String::new(),
            search: String::new(),
            hash: String::new(),
        })
    }

    pub fn host_str(&self) -> Option<&str> {
        Some(&self.host)
    }

    pub fn join(&self, _relative: &str) -> Result<Self, String> {
        Ok(self.clone())
    }

    pub fn origin(&self) -> String {
        self.origin.clone()
    }

    pub fn scheme(&self) -> &str {
        &self.protocol
    }

    pub fn port_or_known_default(&self) -> Option<u16> {
        None
    }

    pub fn path(&self) -> String {
        self.pathname.clone()
    }

    pub fn query(&self) -> Option<&str> {
        if self.search.is_empty() {
            None
        } else {
            Some(self.search.strip_prefix('?').unwrap_or(&self.search))
        }
    }

    pub fn fragment(&self) -> Option<&str> {
        if self.hash.is_empty() {
            None
        } else {
            Some(self.hash.strip_prefix('#').unwrap_or(&self.hash))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.href
    }

    pub fn port(&self) -> Option<u16> {
        self.port.parse().ok()
    }
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.href)
    }
}

pub fn parse(s: &str, base: Option<&Url>) -> Result<Url, String> {
    Url::parse(s, base)
}

pub mod percent_encoding {
    pub enum EncodeSet {
        Query,
    }
    pub fn encode(s: &str, _set: EncodeSet) -> String {
        s.to_string()
    }
    pub fn decode(s: &str) -> String {
        s.to_string()
    }
}
