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

pub fn parse(input: &str, base: Option<&Url>) -> Result<Url, UrlError> {
    let mut url = Url {
        scheme: String::new(),
        username: String::new(),
        password: None,
        host: None,
        port: None,
        path: Vec::new(),
        query: None,
        fragment: None,
    };

    let input = input.trim();
    let mut state = State::SchemeStart;
    let mut buffer = String::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        let mut advance = true;
        match state {
            State::SchemeStart => {
                if c.is_ascii_alphabetic() {
                    buffer.push(c.to_ascii_lowercase());
                    state = State::Scheme;
                } else if let Some(base_url) = base {
                    url.scheme = base_url.scheme.clone();
                    state = State::NoScheme;
                    advance = false;
                } else {
                    return Err(UrlError::MissingScheme);
                }
            }
            State::Scheme => {
                if c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.' {
                    buffer.push(c.to_ascii_lowercase());
                } else if c == ':' {
                    url.scheme = buffer.clone();
                    buffer.clear();
                    if url.scheme == "file" {
                        state = State::File;
                    } else if url.is_special()
                        && base.is_some()
                        && base.unwrap().scheme == url.scheme
                    {
                        state = State::SpecialRelativeOrAuthority;
                    } else if url.is_special() {
                        state = State::SpecialAuthoritySlashes;
                    } else {
                        state = State::PathStart;
                    }
                } else if let Some(base_url) = base {
                    url.scheme = base_url.scheme.clone();
                    state = State::NoScheme;
                    advance = false;
                } else {
                    return Err(UrlError::InvalidScheme);
                }
            }
            State::NoScheme => {
                if let Some(base_url) = base {
                    url.scheme = base_url.scheme.clone();
                    if c == '/' || (url.is_special() && c == '\\') {
                        if i + 1 < chars.len()
                            && (chars[i + 1] == '/' || (url.is_special() && chars[i + 1] == '\\'))
                        {
                            // '//' - override authority entirely (do NOT inherit base host/port)
                            url.path.clear();
                            state = State::SpecialAuthoritySlashes;
                            advance = false;
                        } else {
                            // Single '/' - root-relative, inherit authority but reset path
                            url.username = base_url.username.clone();
                            url.password = base_url.password.clone();
                            url.host = base_url.host.clone();
                            url.port = base_url.port;
                            url.path.clear();
                            state = State::PathStart;
                            advance = false;
                        }
                    } else if c == '?' {
                        url.username = base_url.username.clone();
                        url.password = base_url.password.clone();
                        url.host = base_url.host.clone();
                        url.port = base_url.port;
                        url.path = base_url.path.clone();
                        url.query = Some(String::new());
                        url.fragment = base_url.fragment.clone(); // Inherit fragment
                        state = State::Query;
                    } else if c == '#' {
                        url.username = base_url.username.clone();
                        url.password = base_url.password.clone();
                        url.host = base_url.host.clone();
                        url.port = base_url.port;
                        url.path = base_url.path.clone();
                        url.query = base_url.query.clone();
                        url.fragment = Some(String::new());
                        state = State::Fragment;
                    } else {
                        // Relative path: copy base authority and path up to last segment
                        url.username = base_url.username.clone();
                        url.password = base_url.password.clone();
                        url.host = base_url.host.clone();
                        url.port = base_url.port;
                        url.path = base_url.path.clone();
                        url.query = base_url.query.clone(); // Inherit query
                        url.fragment = base_url.fragment.clone(); // Inherit fragment
                        if !url.path.is_empty() {
                            url.path.pop();
                        }
                        state = State::Path;
                        advance = false;
                    }
                } else {
                    return Err(UrlError::MissingScheme);
                }
            }
            State::SpecialRelativeOrAuthority => {
                if c == '/' && i + 1 < chars.len() && (chars[i + 1] == '/' || chars[i + 1] == '\\')
                {
                    state = State::SpecialAuthoritySlashes;
                    // advance=true: consume this '/', SpecialAuthoritySlashes will consume the next one
                } else {
                    state = State::Relative;
                    advance = false;
                }
            }
            State::Relative => {
                if let Some(base_url) = base {
                    url.scheme = base_url.scheme.clone();
                    if c == '/' || (url.is_special() && c == '\\') {
                        state = State::RelativeSlash;
                    } else if c == '?' {
                        url.username = base_url.username.clone();
                        url.password = base_url.password.clone();
                        url.host = base_url.host.clone();
                        url.port = base_url.port;
                        url.path = base_url.path.clone();
                        url.query = Some(String::new());
                        state = State::Query;
                    } else if c == '#' {
                        url.username = base_url.username.clone();
                        url.password = base_url.password.clone();
                        url.host = base_url.host.clone();
                        url.port = base_url.port;
                        url.path = base_url.path.clone();
                        url.query = base_url.query.clone();
                        url.fragment = Some(String::new());
                        state = State::Fragment;
                    } else {
                        url.username = base_url.username.clone();
                        url.password = base_url.password.clone();
                        url.host = base_url.host.clone();
                        url.port = base_url.port;
                        url.path = base_url.path.clone();
                        if !url.path.is_empty() {
                            url.path.pop();
                        }
                        state = State::Path;
                        advance = false;
                    }
                }
            }
            State::RelativeSlash => {
                if url.is_special() && (c == '/' || c == '\\') {
                    state = State::SpecialAuthoritySlashes;
                } else {
                    url.username = base.unwrap().username.clone();
                    url.password = base.unwrap().password.clone();
                    url.host = base.unwrap().host.clone();
                    url.port = base.unwrap().port;
                    state = State::Path;
                    advance = false;
                }
            }
            State::File => {
                url.scheme = "file".to_string();
                url.host = Some(super::types::Host::Empty);
                if c == '/' || c == '\\' {
                    state = State::FileSlash;
                } else if let Some(base_url) = base {
                    if base_url.scheme == "file" {
                        url.host = base_url.host.clone();
                        url.path = base_url.path.clone();
                        url.query = base_url.query.clone();
                        if c == '?' {
                            url.query = Some(String::new());
                            state = State::Query;
                        } else if c == '#' {
                            url.fragment = Some(String::new());
                            state = State::Fragment;
                        } else {
                            if !url.path.is_empty() {
                                url.path.pop();
                            }
                            state = State::Path;
                            advance = false;
                        }
                    } else {
                        state = State::Path;
                        advance = false;
                    }
                } else {
                    state = State::Path;
                    advance = false;
                }
            }
            State::FileSlash => {
                if c == '/' || c == '\\' {
                    state = State::FileHost;
                } else {
                    if let Some(base_url) = base {
                        if base_url.scheme == "file" {
                            url.host = base_url.host.clone();
                        }
                    }
                    state = State::Path;
                    advance = false;
                }
            }
            State::FileHost => {
                if c == '/' || c == '\\' || c == '?' || c == '#' {
                    state = State::PathStart;
                    advance = false;
                } else {
                    buffer.push(c);
                    // Simplify: handle as hostname if it doesn't look like drive letter
                    // In real WHATWG, we should check for Windows drive letters here
                }
            }
            State::SpecialAuthoritySlashes => {
                // Consume all leading slashes (both '/' of '//')
                if c == '/' || c == '\\' {
                    // just skip
                } else {
                    state = State::SpecialAuthorityIgnoreSlashes;
                    advance = false;
                }
            }
            State::SpecialAuthorityIgnoreSlashes => {
                if c != '/' && c != '\\' {
                    state = State::Authority;
                    advance = false;
                }
            }
            State::Authority => {
                if c == '@' {
                    let user_pass = buffer.clone();
                    if let Some(colon_idx) = user_pass.find(':') {
                        url.username = percent_encoding::decode(&user_pass[..colon_idx]);
                        url.password = Some(percent_encoding::decode(&user_pass[colon_idx + 1..]));
                    } else {
                        url.username = percent_encoding::decode(&user_pass);
                    }
                    buffer.clear();
                    state = State::Host;
                } else if c == '/' || c == '\\' || c == '?' || c == '#' {
                    // Authority ended without '@'. Buffer contains 'host' or 'host:port'
                    let auth_str = buffer.clone();
                    buffer.clear();
                    // Handle IPv6: if buffer starts with '[', find ']' first
                    if auth_str.starts_with('[') {
                        if let Some(bracket_end) = auth_str.find(']') {
                            let ipv6_part = &auth_str[1..bracket_end];
                            if let Ok(addr) = ipv6_part.parse::<std::net::Ipv6Addr>() {
                                url.host = Some(super::types::Host::Ipv6(addr));
                            } else {
                                url.host = Some(parse_host(&auth_str[..=bracket_end]));
                            }
                            // Check for port after ']'
                            let after_bracket = &auth_str[bracket_end + 1..];
                            if let Some(port_str) = after_bracket.strip_prefix(':') {
                                url.port = port_str.parse().ok();
                            }
                        } else {
                            url.host = Some(parse_host(&auth_str));
                        }
                    } else if let Some(colon_idx) = auth_str.find(':') {
                        let host_part = &auth_str[..colon_idx];
                        let port_part = &auth_str[colon_idx + 1..];
                        url.host = Some(parse_host(host_part));
                        if !port_part.is_empty() {
                            url.port = port_part.parse().ok();
                        }
                    } else {
                        url.host = Some(parse_host(&auth_str));
                    }
                    state = State::PathStart;
                    advance = false;
                } else {
                    buffer.push(c);
                }
            }
            State::Host => {
                if c == '[' {
                    // Start of IPv6
                    buffer.clear();
                    state = State::Ipv6;
                } else if c == ':' {
                    let host_str = buffer.clone();
                    url.host = Some(parse_host(&host_str));
                    buffer.clear();
                    state = State::Port;
                } else if c == '/' || (url.is_special() && c == '\\') || c == '?' || c == '#' {
                    let host_str = buffer.clone();
                    url.host = Some(parse_host(&host_str));
                    buffer.clear();
                    state = State::PathStart;
                    advance = false;
                } else {
                    buffer.push(c.to_ascii_lowercase());
                }
            }
            State::Ipv6 => {
                if c == ']' {
                    let ipv6_str = buffer.clone();
                    if let Ok(addr) = ipv6_str.parse::<std::net::Ipv6Addr>() {
                        url.host = Some(super::types::Host::Ipv6(addr));
                    } else {
                        return Err(UrlError::InvalidHost);
                    }
                    buffer.clear();
                    state = State::Port;
                } else if c.is_ascii_hexdigit() || c == ':' || c == '.' {
                    buffer.push(c);
                } else {
                    return Err(UrlError::InvalidHost);
                }
            }
            State::Port => {
                if c.is_ascii_digit() {
                    buffer.push(c);
                } else {
                    if !buffer.is_empty() {
                        // println!("DEBUG: Port ending, buffer='{}'", buffer);
                        url.port = buffer.parse().ok();
                    }
                    buffer.clear();
                    state = State::PathStart;
                    advance = false;
                }
            }
            State::PathStart => {
                state = State::Path;
                if c != '/' && c != '\\' {
                    advance = false;
                }
            }
            State::Path => {
                if c == '/'
                    || (url.is_special() && c == '\\')
                    || (i + 1 == chars.len() && !buffer.is_empty())
                    || c == '?'
                    || c == '#'
                {
                    if c != '?' && c != '#' && i + 1 == chars.len() && !buffer.is_empty() {
                        buffer.push(c);
                    }

                    if !buffer.is_empty() {
                        let decoded = percent_encoding::decode(&buffer);
                        if decoded == ".." {
                            url.path.pop();
                            if c == '/' || (url.is_special() && c == '\\') {
                                // WHATWG: se termina em /.. deve garantir que o path termine em /
                            }
                        } else if decoded != "." {
                            url.path.push(percent_encoding::encode(
                                &decoded,
                                percent_encoding::EncodeSet::Path,
                            ));
                        }
                        buffer.clear();
                    }

                    if c == '?' {
                        state = State::Query;
                    } else if c == '#' {
                        state = State::Fragment;
                    }
                } else {
                    buffer.push(c);
                }
            }
            State::Query => {
                if c == '#' {
                    url.query = Some(percent_encoding::encode(
                        &buffer,
                        percent_encoding::EncodeSet::Query,
                    ));
                    buffer.clear();
                    state = State::Fragment;
                } else {
                    buffer.push(c);
                }
            }
            State::Fragment => {
                buffer.push(c);
            }
            _ => {}
        }
        if advance {
            i += 1;
        }
    }

    // Final buffers
    match state {
        State::Scheme => {
            if let Some(base_url) = base {
                // Input como "d" (sem ':') com base deve ser tratado como URL relativa.
                url.scheme = base_url.scheme.clone();
                url.username = base_url.username.clone();
                url.password = base_url.password.clone();
                url.host = base_url.host.clone();
                url.port = base_url.port;
                url.path = base_url.path.clone();
                url.query = base_url.query.clone();
                url.fragment = base_url.fragment.clone();
                if !url.path.is_empty() {
                    url.path.pop();
                }
                if !buffer.is_empty() {
                    let decoded = percent_encoding::decode(&buffer);
                    if decoded == ".." {
                        url.path.pop();
                    } else if decoded != "." {
                        url.path.push(percent_encoding::encode(
                            &decoded,
                            percent_encoding::EncodeSet::Path,
                        ));
                    }
                }
            } else {
                return Err(UrlError::MissingScheme);
            }
        }
        State::Query => {
            url.query = Some(percent_encoding::encode(
                &buffer,
                percent_encoding::EncodeSet::Query,
            ))
        }
        State::Fragment => {
            url.fragment = Some(percent_encoding::encode(
                &buffer,
                percent_encoding::EncodeSet::Fragment,
            ))
        }
        State::Host | State::Ipv6 | State::Port => {
            if state == State::Port && !buffer.is_empty() {
                url.port = buffer.parse().ok();
            } else if state == State::Host && !buffer.is_empty() {
                url.host = Some(parse_host(&buffer));
            }
        }
        State::Path => {
            if !buffer.is_empty() {
                let decoded = percent_encoding::decode(&buffer);
                if decoded == ".." {
                    url.path.pop();
                } else if decoded != "." {
                    url.path.push(percent_encoding::encode(
                        &decoded,
                        percent_encoding::EncodeSet::Path,
                    ));
                }
            }
        }
        _ => {}
    }

    Ok(url)
}

fn parse_host(input: &str) -> super::types::Host {
    if input.is_empty() {
        return super::types::Host::Empty;
    }

    // Try IPv4
    if let Ok(addr) = input.parse::<std::net::Ipv4Addr>() {
        return super::types::Host::Ipv4(addr);
    }

    // Domain (with Punycode if needed)
    let domain = input.to_lowercase();
    let idn_domain = if domain.chars().any(|c| c > '\x7F') {
        punycode::encode(&domain).unwrap_or_else(|_| domain.clone())
    } else {
        domain
    };

    super::types::Host::Domain(idn_domain)
}
