use super::*;
use super::percent_encoding;
use super::punycode;
use super::types::{Url, UrlError};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
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
