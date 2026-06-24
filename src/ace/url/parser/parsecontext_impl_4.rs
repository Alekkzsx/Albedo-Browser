use super::*;
use super::percent_encoding;
use super::punycode;
use super::types::{Url, UrlError};



impl ParseContext {

pub(crate) fn handle_path_state(&mut self) {
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

pub(crate) fn finalize(&mut self) -> Result<(), UrlError> {
        match self.state {
            State::Scheme => {
                if let Some(ref base_url) = self.base.clone() {
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

pub(crate) fn step(&mut self) -> Result<(), UrlError> {
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
