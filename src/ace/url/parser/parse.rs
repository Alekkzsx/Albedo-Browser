use super::*;
use super::percent_encoding;
use super::punycode;
use super::types::{Url, UrlError};



/// TODO: add docs
pub fn parse(input: &str, base: Option<&Url>) -> Result<Url, UrlError> {
    let mut ctx = ParseContext::new(input, base);
    while ctx.i < ctx.chars.len() {
        ctx.step()?;
        ctx.i += 1;
    }
    ctx.finalize()?;
    Ok(ctx.url)
}
