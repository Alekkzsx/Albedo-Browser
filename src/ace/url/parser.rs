use super::types::{Url, UrlError};
use super::percent_encoding;
use super::punycode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    SchemeStart,
    Scheme,
    NoScheme,
    SpecialRelativeOrAuthority,
    PathOrAuthority,
    Relative,
    RelativeSlash,
    SpecialAuthoritySlashes,
    SpecialAuthorityIgnoreSlashes,
    Authority,
    Host,
    Hostname,
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
        match state {
            State::SchemeStart => {
                if c.is_ascii_alphabetic() {
                    buffer.push(c.to_ascii_lowercase());
                    state = State::Scheme;
                } else if let Some(base_url) = base {
                    url.scheme = base_url.scheme.clone();
                    state = State::NoScheme;
                    i = i.saturating_sub(1);
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
                    } else if url.is_special() && base.is_some() && base.unwrap().scheme == url.scheme {
                        state = State::SpecialRelativeOrAuthority;
                    } else if url.is_special() {
                        state = State::SpecialAuthoritySlashes;
                    } else {
                        state = State::PathStart;
                    }
                } else if let Some(base_url) = base {
                    url.scheme = base_url.scheme.clone();
                    state = State::NoScheme;
                    i = i.saturating_sub(1);
                } else {
                    return Err(UrlError::InvalidScheme);
                }
            }
            State::NoScheme => {
                if let Some(base_url) = base {
                    url.scheme = base_url.scheme.clone();
                    if c == '/' || (url.is_special() && c == '\\') {
                        // Inherit authority, reset path
                        url.username = base_url.username.clone();
                        url.password = base_url.password.clone();
                        url.host = base_url.host.clone();
                        url.port = base_url.port;
                        url.query = base_url.query.clone(); // Inherit query
                        url.fragment = base_url.fragment.clone(); // Inherit fragment
                        
                        if i + 1 < chars.len() && (chars[i+1] == '/' || (url.is_special() && chars[i+1] == '\\')) {
                            state = State::SpecialAuthoritySlashes;
                            i += 1;
                        } else {
                            url.path.clear();
                            state = State::PathStart;
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
                        i -= 1;
                    }
                } else {
                    return Err(UrlError::MissingScheme);
                }
            }
            State::SpecialAuthoritySlashes => {
                if c == '/' && i + 1 < chars.len() && chars[i+1] == '/' {
                    i += 1;
                    state = State::SpecialAuthorityIgnoreSlashes;
                } else {
                    state = State::SpecialAuthorityIgnoreSlashes;
                    if i > 0 { i -= 1; }
                }
            }
            State::SpecialAuthorityIgnoreSlashes => {
                if c != '/' && c != '\\' {
                    state = State::Authority;
                    if i > 0 { i -= 1; }
                }
            }
            State::Authority => {
                if c == '@' {
                    let user_pass = buffer.clone();
                    if let Some(colon_idx) = user_pass.find(':') {
                        url.username = percent_encoding::decode(&user_pass[..colon_idx]);
                        url.password = Some(percent_encoding::decode(&user_pass[colon_idx+1..]));
                    } else {
                        url.username = percent_encoding::decode(&user_pass);
                    }
                    buffer.clear();
                } else if c == '/' || c == '\\' || c == '?' || c == '#' {
                    state = State::Host;
                    if i > 0 { i -= 1; }
                } else {
                    buffer.push(c);
                }
            }
            State::Host | State::Hostname => {
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
                    if i > 0 { i -= 1; }
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
                        url.port = buffer.parse().ok();
                    }
                    buffer.clear();
                    state = State::PathStart;
                    if i > 0 { i -= 1; }
                }
            }
            State::PathStart => {
                state = State::Path;
                if c != '/' && c != '\\' {
                    if i > 0 { i -= 1; }
                }
            }
            State::Path => {
                if c == '/' || (url.is_special() && c == '\\') || (i + 1 == chars.len() && !buffer.is_empty()) || c == '?' || c == '#' {
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
                            url.path.push(percent_encoding::encode(&decoded, percent_encoding::EncodeSet::Path));
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
                    url.query = Some(percent_encoding::encode(&buffer, percent_encoding::EncodeSet::Query));
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
        i += 1;
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
        State::Query => url.query = Some(percent_encoding::encode(&buffer, percent_encoding::EncodeSet::Query)),
        State::Fragment => url.fragment = Some(percent_encoding::encode(&buffer, percent_encoding::EncodeSet::Fragment)),
        State::Host | State::Hostname => {
             url.host = Some(parse_host(&buffer));
        },
        State::Port => if !buffer.is_empty() { url.port = buffer.parse().ok(); },
        State::Path => if !buffer.is_empty() {
            let decoded = percent_encoding::decode(&buffer);
            if decoded == ".." {
                url.path.pop();
            } else if decoded != "." {
                url.path.push(percent_encoding::encode(&decoded, percent_encoding::EncodeSet::Path));
            }
        },
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
