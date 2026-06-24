use super::*;
use super::percent_encoding;
use super::punycode;
use super::types::{Url, UrlError};



impl ParseContext {

pub(crate) fn handle_scheme(&mut self) -> Result<(), UrlError> {
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
                && self.base.expect("Albedo Engine: internal invariant violated").scheme == self.url.scheme
            {
                self.state = State::SpecialRelativeOrAuthority;
            } else if self.url.is_special() {
                self.state = State::SpecialAuthoritySlashes;
            } else {
                self.state = State::PathStart;
            }
        } else if let Some(base_url) = self.base.clone() {
            self.url.scheme = base_url.scheme.clone();
            self.state = State::NoScheme;
            self.i -= 1;
        } else {
            return Err(UrlError::InvalidScheme);
        }
        Ok(())
    }

pub(crate) fn handle_no_scheme(&mut self) -> Result<(), UrlError> {
        let c = self.c();
        if let Some(base_url) = self.base.clone() {
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

pub(crate) fn handle_relative_states(&mut self) -> Result<(), UrlError> {
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
                if let Some(base_url) = self.base.clone() {
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
}
