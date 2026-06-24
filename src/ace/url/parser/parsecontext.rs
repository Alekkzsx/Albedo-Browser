use super::*;
use super::percent_encoding;
use super::punycode;
use super::types::{Url, UrlError};



pub(crate) struct ParseContext {
    url: Url,
    buffer: String,
    state: State,
    chars: Vec<char>,
    i: usize,
    base: Option<Url>,
}
