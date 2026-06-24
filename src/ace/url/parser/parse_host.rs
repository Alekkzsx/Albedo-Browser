use super::*;
use super::percent_encoding;
use super::punycode;
use super::types::{Url, UrlError};



pub(crate) fn parse_host(input: &str) -> super::types::Host {
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
