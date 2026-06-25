use super::*;
use super::percent_encoding;
use super::punycode;
use super::types::{Url, UrlError};



pub(crate) struct ParseContext {
    pub url: Url,
    pub buffer: String,
    pub state: State,
    pub chars: Vec<char>,
    pub i: usize,
    pub base: Option<Url>,
}
