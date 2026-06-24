use super::*;
use super::percent_encoding;
use super::punycode;
use super::types::{Url, UrlError};



impl ParseContext {
pub(crate) fn new(input: &str, base: Option<&Url>) -> Self {
        Self {
            url: Url {
                scheme: String::new(),
                username: String::new(),
                password: None,
                host: None,
                port: None,
                path: Vec::new(),
                query: None,
                fragment: None,
            },
            buffer: String::new(),
            state: State::SchemeStart,
            chars: input.chars().collect(),
            i: 0,
            base: base.cloned(),
        }
    }

pub(crate) fn c(&self) -> char {
        self.chars[self.i]
    }

pub(crate) fn inherit_base(&mut self, include_path: bool, include_query: bool) {
        let base = self.base.clone().expect("Albedo Engine: internal invariant violated");
        self.url.username = base.username.clone();
        self.url.password = base.password.clone();
        self.url.host = base.host.clone();
        self.url.port = base.port;
        if include_path {
            self.url.path = base.path.clone();
        }
        if include_query {
            self.url.query = base.query.clone();
        }
    }

pub(crate) fn inherit_base_full(&mut self) {
        let base = self.base.clone().expect("Albedo Engine: internal invariant violated");
        self.url.scheme = base.scheme.clone();
        self.inherit_base(true, true);
        self.url.fragment = base.fragment.clone();
    }

pub(crate) fn parse_authority_from_buffer(&mut self) {
        let auth_str = self.buffer.clone();
        if auth_str.starts_with('[') {
            if let Some(bracket_end) = auth_str.find(']') {
                let ipv6_part = &auth_str[1..bracket_end];
                if let Ok(addr) = ipv6_part.parse::<std::net::Ipv6Addr>() {
                    self.url.host = Some(super::types::Host::Ipv6(addr));
                } else {
                    self.url.host = Some(parse_host(&auth_str[..=bracket_end]));
                }
                let after_bracket = &auth_str[bracket_end + 1..];
                if let Some(port_str) = after_bracket.strip_prefix(':') {
                    self.url.port = port_str.parse().ok();
                }
            } else {
                self.url.host = Some(parse_host(&auth_str));
            }
        } else if let Some(colon_idx) = auth_str.find(':') {
            let host_part = &auth_str[..colon_idx];
            let port_part = &auth_str[colon_idx + 1..];
            self.url.host = Some(parse_host(host_part));
            if !port_part.is_empty() {
                self.url.port = port_part.parse().ok();
            }
        } else {
            self.url.host = Some(parse_host(&auth_str));
        }
    }

pub(crate) fn push_path_segment(&mut self) {
        let decoded = percent_encoding::decode(&self.buffer);
        if decoded == ".." {
            self.url.path.pop();
        } else if decoded != "." {
            self.url.path.push(percent_encoding::encode(
                &decoded,
                percent_encoding::EncodeSet::Path,
            ));
        }
    }

pub(crate) fn handle_scheme_start(&mut self) -> Result<(), UrlError> {
        let c = self.c();
        if c.is_ascii_alphabetic() {
            self.buffer.push(c.to_ascii_lowercase());
            self.state = State::Scheme;
        } else if let Some(base_url) = self.base.clone() {
            self.url.scheme = base_url.scheme.clone();
            self.state = State::NoScheme;
            self.i -= 1;
        } else {
            return Err(UrlError::MissingScheme);
        }
        Ok(())
    }
}
