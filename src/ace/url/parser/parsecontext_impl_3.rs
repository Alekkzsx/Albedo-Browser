use super::*;
use super::percent_encoding;
use super::punycode;
use super::types::{Url, UrlError};



impl ParseContext {

pub(crate) fn handle_file_states(&mut self) -> Result<(), UrlError> {
        let c = self.c();
        match self.state {
            State::File => {
                self.url.scheme = "file".to_string();
                self.url.host = Some(super::types::Host::Empty);
                if c == '/' || c == '\\' {
                    self.state = State::FileSlash;
                } else if let Some(base_url) = self.base.clone() {
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
                    if let Some(base_url) = self.base.clone() {
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

pub(crate) fn handle_authority_state(&mut self) -> Result<(), UrlError> {
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

pub(crate) fn handle_host_states(&mut self) -> Result<(), UrlError> {
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
}
