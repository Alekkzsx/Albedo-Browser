use super::percent_encoding;
use super::punycode;
use super::types::{Url, UrlError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    SchemeStart,
    Scheme,
    NoScheme,
    SpecialRelativeOrAuthority,
    Relative,
    RelativeSlash,
    SpecialAuthoritySlashes,
    SpecialAuthorityIgnoreSlashes,
    Authority,
    Host,
    Ipv6,
    Port,
    File,
    FileSlash,
    FileHost,
    PathStart,
    Path,
    Query,
    Fragment,
}

struct ParseContext {
    url: Url,
    buffer: String,
    state: State,
    chars: Vec<char>,
    i: usize,
    base: Option<Url>,
}

impl ParseContext {
    fn new(input: &str, base: Option<&Url>) -> Self {
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

    fn c(&self) -> char {
        self.chars[self.i]
    }

    fn inherit_base(&mut self, include_path: bool, include_query: bool) {
        let base = self.base.as_ref().unwrap();
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

    fn inherit_base_full(&mut self) {
        let base = self.base.as_ref().unwrap();
        self.url.scheme = base.scheme.clone();
        self.inherit_base(true, true);
        self.url.fragment = base.fragment.clone();
    }

    fn parse_authority_from_buffer(&mut self) {
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

    fn push_path_segment(&mut self) {
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

    fn handle_scheme_start(&mut self) -> Result<(), UrlError> {
        let c = self.c();
        if c.is_ascii_alphabetic() {
            self.buffer.push(c.to_ascii_lowercase());
            self.state = State::Scheme;
        } else if let Some(base_url) = self.base {
            self.url.scheme = base_url.scheme.clone();
            self.state = State::NoScheme;
            self.i -= 1;
        } else {
            return Err(UrlError::MissingScheme);
        }
        Ok(())
    }

    fn handle_scheme(&mut self) -> Result<(), UrlError> {
        let c = self.c();
        if c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.' {
            self.buffer.push(c.to_ascii_lowercase());
        } else if c == ':' {
            self.url.scheme = self.buffer.clone();
            self.buffer.clear();
            if self.url.scheme == "file" {
                self.state = State::File;
            } else if self.url.is_special()
                && self.base.is_some()
                && self.base.unwrap().scheme == self.url.scheme
            {
                self.state = State::SpecialRelativeOrAuthority;
            } else if self.url.is_special() {
                self.state = State::SpecialAuthoritySlashes;
            } else {
                self.state = State::PathStart;
            }
        } else if let Some(base_url) = self.base {
            self.url.scheme = base_url.scheme.clone();
            self.state = State::NoScheme;
            self.i -= 1;
        } else {
            return Err(UrlError::InvalidScheme);
        }
        Ok(())
    }

    fn handle_no_scheme(&mut self) -> Result<(), UrlError> {
        let c = self.c();
        if let Some(base_url) = self.base {
            self.url.scheme = base_url.scheme.clone();
            if c == '/' || (self.url.is_special() && c == '\\') {
                if self.i + 1 < self.chars.len()
                    && (self.chars[self.i + 1] == '/'
                        || (self.url.is_special() && self.chars[self.i + 1] == '\\'))
                {
                    self.url.path.clear();
                    self.state = State::SpecialAuthoritySlashes;
                    self.i -= 1;
                } else {
                    self.inherit_base(false, false);
                    self.url.path.clear();
                    self.state = State::PathStart;
                    self.i -= 1;
                }
            } else if c == '?' {
                self.inherit_base(true, false);
                self.url.query = Some(String::new());
                self.url.fragment = base_url.fragment.clone();
                self.state = State::Query;
            } else if c == '#' {
                self.inherit_base(true, true);
                self.url.fragment = Some(String::new());
                self.state = State::Fragment;
            } else {
                self.inherit_base(true, true);
                self.url.fragment = base_url.fragment.clone();
                if !self.url.path.is_empty() {
                    self.url.path.pop();
                }
                self.state = State::Path;
                self.i -= 1;
            }
        } else {
            return Err(UrlError::MissingScheme);
        }
        Ok(())
    }

    fn handle_relative_states(&mut self) -> Result<(), UrlError> {
        let c = self.c();
        match self.state {
            State::SpecialRelativeOrAuthority => {
                if c == '/'
                    && self.i + 1 < self.chars.len()
                    && (self.chars[self.i + 1] == '/' || self.chars[self.i + 1] == '\\')
                {
                    self.state = State::SpecialAuthoritySlashes;
                } else {
                    self.state = State::Relative;
                    self.i -= 1;
                }
            }
            State::Relative => {
                if let Some(base_url) = self.base {
                    self.url.scheme = base_url.scheme.clone();
                    if c == '/' || (self.url.is_special() && c == '\\') {
                        self.state = State::RelativeSlash;
                    } else if c == '?' {
                        self.inherit_base(true, false);
                        self.url.query = Some(String::new());
                        self.state = State::Query;
                    } else if c == '#' {
                        self.inherit_base(true, true);
                        self.url.fragment = Some(String::new());
                        self.state = State::Fragment;
                    } else {
                        self.inherit_base(true, false);
                        if !self.url.path.is_empty() {
                            self.url.path.pop();
                        }
                        self.state = State::Path;
                        self.i -= 1;
                    }
                }
            }
            State::RelativeSlash => {
                if self.url.is_special() && (c == '/' || c == '\\') {
                    self.state = State::SpecialAuthoritySlashes;
                } else {
                    self.inherit_base(false, false);
                    self.state = State::Path;
                    self.i -= 1;
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    fn handle_file_states(&mut self) -> Result<(), UrlError> {
        let c = self.c();
        match self.state {
            State::File => {
                self.url.scheme = "file".to_string();
                self.url.host = Some(super::types::Host::Empty);
                if c == '/' || c == '\\' {
                    self.state = State::FileSlash;
                } else if let Some(base_url) = self.base {
                    if base_url.scheme == "file" {
                        self.url.host = base_url.host.clone();
                        self.url.path = base_url.path.clone();
                        self.url.query = base_url.query.clone();
                        if c == '?' {
                            self.url.query = Some(String::new());
                            self.state = State::Query;
                        } else if c == '#' {
                            self.url.fragment = Some(String::new());
                            self.state = State::Fragment;
                        } else {
                            if !self.url.path.is_empty() {
                                self.url.path.pop();
                            }
                            self.state = State::Path;
                            self.i -= 1;
                        }
                    } else {
                        self.state = State::Path;
                        self.i -= 1;
                    }
                } else {
                    self.state = State::Path;
                    self.i -= 1;
                }
            }
            State::FileSlash => {
                if c == '/' || c == '\\' {
                    self.state = State::FileHost;
                } else {
                    if let Some(base_url) = self.base {
                        if base_url.scheme == "file" {
                            self.url.host = base_url.host.clone();
                        }
                    }
                    self.state = State::Path;
                    self.i -= 1;
                }
            }
            State::FileHost => {
                if c == '/' || c == '\\' || c == '?' || c == '#' {
                    self.state = State::PathStart;
                    self.i -= 1;
                } else {
                    self.buffer.push(c);
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    fn handle_authority_state(&mut self) -> Result<(), UrlError> {
        let c = self.c();
        if c == '@' {
            let user_pass = self.buffer.clone();
            if let Some(colon_idx) = user_pass.find(':') {
                self.url.username = percent_encoding::decode(&user_pass[..colon_idx]);
                self.url.password =
                    Some(percent_encoding::decode(&user_pass[colon_idx + 1..]));
            } else {
                self.url.username = percent_encoding::decode(&user_pass);
            }
            self.buffer.clear();
            self.state = State::Host;
        } else if c == '/' || c == '\\' || c == '?' || c == '#' {
            self.parse_authority_from_buffer();
            self.buffer.clear();
            self.state = State::PathStart;
            self.i -= 1;
        } else {
            self.buffer.push(c);
        }
        Ok(())
    }

    fn handle_host_states(&mut self) -> Result<(), UrlError> {
        let c = self.c();
        match self.state {
            State::Host => {
                if c == '[' {
                    self.buffer.clear();
                    self.state = State::Ipv6;
                } else if c == ':' {
                    let host_str = self.buffer.clone();
                    self.url.host = Some(parse_host(&host_str));
                    self.buffer.clear();
                    self.state = State::Port;
                } else if c == '/'
                    || (self.url.is_special() && c == '\\')
                    || c == '?'
                    || c == '#'
                {
                    let host_str = self.buffer.clone();
                    self.url.host = Some(parse_host(&host_str));
                    self.buffer.clear();
                    self.state = State::PathStart;
                    self.i -= 1;
                } else {
                    self.buffer.push(c.to_ascii_lowercase());
                }
            }
            State::Ipv6 => {
                if c == ']' {
                    let ipv6_str = self.buffer.clone();
                    if let Ok(addr) = ipv6_str.parse::<std::net::Ipv6Addr>() {
                        self.url.host = Some(super::types::Host::Ipv6(addr));
                    } else {
                        return Err(UrlError::InvalidHost);
                    }
                    self.buffer.clear();
                    self.state = State::Port;
                } else if c.is_ascii_hexdigit() || c == ':' || c == '.' {
                    self.buffer.push(c);
                } else {
                    return Err(UrlError::InvalidHost);
                }
            }
            State::Port => {
                if c.is_ascii_digit() {
                    self.buffer.push(c);
                } else {
                    if !self.buffer.is_empty() {
                        self.url.port = self.buffer.parse().ok();
                    }
                    self.buffer.clear();
                    self.state = State::PathStart;
                    self.i -= 1;
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    fn handle_path_state(&mut self) {
        let c = self.c();
        if c == '/'
            || (self.url.is_special() && c == '\\')
            || (self.i + 1 == self.chars.len() && !self.buffer.is_empty())
            || c == '?'
            || c == '#'
        {
            if c != '?' && c != '#' && self.i + 1 == self.chars.len() && !self.buffer.is_empty() {
                self.buffer.push(c);
            }

            if !self.buffer.is_empty() {
                self.push_path_segment();
                self.buffer.clear();
            }

            if c == '?' {
                self.state = State::Query;
            } else if c == '#' {
                self.state = State::Fragment;
            }
        } else {
            self.buffer.push(c);
        }
    }

    fn finalize(&mut self) -> Result<(), UrlError> {
        match self.state {
            State::Scheme => {
                if let Some(ref base_url) = self.base {
                    self.url.scheme = base_url.scheme.clone();
                    self.inherit_base(true, true);
                    self.url.fragment = base_url.fragment.clone();
                    if !self.url.path.is_empty() {
                        self.url.path.pop();
                    }
                    if !self.buffer.is_empty() {
                        self.push_path_segment();
                    }
                } else {
                    return Err(UrlError::MissingScheme);
                }
            }
            State::Query => {
                self.url.query = Some(percent_encoding::encode(
                    &self.buffer,
                    percent_encoding::EncodeSet::Query,
                ));
            }
            State::Fragment => {
                self.url.fragment = Some(percent_encoding::encode(
                    &self.buffer,
                    percent_encoding::EncodeSet::Fragment,
                ));
            }
            State::Host | State::Ipv6 | State::Port => {
                if self.state == State::Port && !self.buffer.is_empty() {
                    self.url.port = self.buffer.parse().ok();
                } else if self.state == State::Host && !self.buffer.is_empty() {
                    self.url.host = Some(parse_host(&self.buffer));
                }
            }
            State::Authority => {
                if !self.buffer.is_empty() {
                    self.parse_authority_from_buffer();
                }
            }
            State::Path => {
                if !self.buffer.is_empty() {
                    self.push_path_segment();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn step(&mut self) -> Result<(), UrlError> {
        match self.state {
            State::SchemeStart => self.handle_scheme_start(),
            State::Scheme => self.handle_scheme(),
            State::NoScheme => self.handle_no_scheme(),
            State::SpecialRelativeOrAuthority | State::Relative | State::RelativeSlash => {
                self.handle_relative_states()
            }
            State::File | State::FileSlash | State::FileHost => self.handle_file_states(),
            State::SpecialAuthoritySlashes => {
                let c = self.c();
                if c == '/' || c == '\\' {
                    Ok(())
                } else {
                    self.state = State::SpecialAuthorityIgnoreSlashes;
                    self.i -= 1;
                    Ok(())
                }
            }
            State::SpecialAuthorityIgnoreSlashes => {
                let c = self.c();
                if c != '/' && c != '\\' {
                    self.state = State::Authority;
                    self.i -= 1;
                }
                Ok(())
            }
            State::Authority => self.handle_authority_state(),
            State::Host | State::Ipv6 | State::Port => self.handle_host_states(),
            State::PathStart => {
                let c = self.c();
                self.state = State::Path;
                if c != '/' && c != '\\' {
                    self.i -= 1;
                }
                Ok(())
            }
            State::Path => {
                self.handle_path_state();
                Ok(())
            }
            State::Query => {
                let c = self.c();
                if c == '#' {
                    self.url.query = Some(percent_encoding::encode(
                        &self.buffer,
                        percent_encoding::EncodeSet::Query,
                    ));
                    self.buffer.clear();
                    self.state = State::Fragment;
                } else {
                    self.buffer.push(c);
                }
                Ok(())
            }
            State::Fragment => {
                self.buffer.push(self.c());
                Ok(())
            }
        }
    }
}

pub fn parse(input: &str, base: Option<&Url>) -> Result<Url, UrlError> {
    let mut ctx = ParseContext::new(input, base);
    while ctx.i < ctx.chars.len() {
        ctx.step()?;
        ctx.i += 1;
    }
    ctx.finalize()?;
    Ok(ctx.url)
}

fn parse_host(input: &str) -> super::types::Host {
    if input.is_empty() {
        return super::types::Host::Empty;
    }

    if let Ok(addr) = input.parse::<std::net::Ipv4Addr>() {
        return super::types::Host::Ipv4(addr);
    }

    let domain = input.to_lowercase();
    let idn_domain = if domain.chars().any(|c| c > '\x7F') {
        punycode::encode(&domain).unwrap_or_else(|_| domain.clone())
    } else {
        domain
    };

    super::types::Host::Domain(idn_domain)
}
