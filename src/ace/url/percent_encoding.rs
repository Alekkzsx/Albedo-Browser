pub fn decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(h1), Some(h2)) = (h1, h2) {
                if let Ok(byte) = u8::from_str_radix(&format!("{}{}", h1, h2), 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            // Fallback se não for hex válido
            result.push('%');
            if let Some(h1) = h1 { result.push(h1); }
            if let Some(h2) = h2 { result.push(h2); }
        } else {
            result.push(c);
        }
    }
    result
}

pub fn encode(input: &str, set: EncodeSet) -> String {
    let mut result = String::with_capacity(input.len());
    for b in input.as_bytes() {
        if should_encode(*b, set) {
            result.push_str(&format!("%{:02X}", b));
        } else {
            result.push(*b as char);
        }
    }
    result
}

#[derive(Debug, Clone, Copy)]
pub enum EncodeSet {
    /// C0 control percent-encode set
    C0Control,
    /// Fragment percent-encode set
    Fragment,
    /// Query percent-encode set
    Query,
    /// Path percent-encode set
    Path,
    /// Userinfo percent-encode set
    UserInfo,
    /// Component percent-encode set
    Component,
}

fn should_encode(b: u8, set: EncodeSet) -> bool {
    if b <= 0x1F || b >= 0x7F {
        return true;
    }
    let c = b as char;
    match set {
        EncodeSet::C0Control => false,
        EncodeSet::Fragment => matches!(c, ' ' | '"' | '<' | '>' | '`'),
        EncodeSet::Query => matches!(c, ' ' | '"' | '#' | '<' | '>'),
        EncodeSet::Path => matches!(c, ' ' | '"' | '#' | '<' | '>' | '?' | '`' | '{' | '}'),
        EncodeSet::UserInfo => matches!(
            c,
            ' ' | '"' | '#' | '<' | '>' | '?' | '`' | '{' | '}' | '/' | ':' | ';' | '=' | '@' | '[' | '\\' | ']' | '^' | '|'
        ),
        EncodeSet::Component => !c.is_ascii_alphanumeric() && !matches!(c, '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')'),
    }
}
