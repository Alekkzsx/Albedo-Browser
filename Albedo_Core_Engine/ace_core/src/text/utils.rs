//! # Utilitários de Texto, Escape e Sanitização
//!
//! Funções de escape para entidades HTML/CSS e comparações case-insensitive de strings.

/// Escapa caracteres especiais para inserção segura em nós de texto ou atributos HTML (`&`, `<`, `>`, `"`, `'`).
pub fn escape_html(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 16);
    for c in raw.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// Escapa identificadores CSS (classes, IDs) conforme a especificação CSSOM.
pub fn escape_css_identifier(ident: &str) -> String {
    let mut out = String::with_capacity(ident.len() + 8);
    for (i, c) in ident.chars().enumerate() {
        if c == '\0' {
            out.push('\u{FFFD}');
        } else if (i == 0 && c.is_ascii_digit())
            || (i == 1 && c.is_ascii_digit() && ident.starts_with('-'))
        {
            out.push_str(&format!("\\{:x} ", c as u32));
        } else if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c > '\u{007F}' {
            out.push(c);
        } else {
            out.push('\\');
            out.push(c);
        }
    }
    out
}

/// Compara duas strings ignorando maiúsculas/minúsculas ASCII de forma rápida.
#[inline]
pub fn is_ascii_case_insensitive_equal(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}
