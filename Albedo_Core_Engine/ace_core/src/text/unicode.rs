//! # Utilitários de Unicode e Mapeamento UTF-8 ↔ UTF-16
//!
//! Funções zero-allocation para conversão de índices e conformidade com especificações
//! do DOM (que utilizam code units UTF-16) e JavaScript (V8/QuickJS `String.length`).

/// Converte um offset em bytes UTF-8 para o offset correspondente em unidades de código UTF-16.
pub fn utf8_byte_to_utf16_offset(text: &str, byte_offset: usize) -> usize {
    let limit = byte_offset.min(text.len());
    let valid_sub = match text.get(..limit) {
        Some(s) => s,
        None => {
            // Em caso de offset cortando no meio de um caractere multibyte, encontra o limite de caractere anterior
            let mut end = limit;
            while end > 0 && !text.is_char_boundary(end) {
                end -= 1;
            }
            &text[..end]
        }
    };

    valid_sub.chars().map(|c| c.len_utf16()).sum()
}

/// Converte um offset em unidades de código UTF-16 para o offset correspondente em bytes UTF-8.
pub fn utf16_offset_to_utf8_byte(text: &str, utf16_offset: usize) -> usize {
    let mut current_utf16 = 0;
    let mut current_bytes = 0;

    for c in text.chars() {
        if current_utf16 >= utf16_offset {
            break;
        }
        current_utf16 += c.len_utf16();
        current_bytes += c.len_utf8();
    }

    current_bytes
}

/// Conta o número total de unidades de código UTF-16 de uma string (equivalente a `String.length` em JS).
#[inline]
pub fn count_utf16_units(text: &str) -> usize {
    text.chars().map(|c| c.len_utf16()).sum()
}

/// Retorna `true` se o caractere for considerado um espaço em branco pela especificação HTML (WHATWG 2.4.2).
///
/// Espaços HTML são estritamente: Space (`0x20`), Tab (`0x09`), LF (`0x0A`), FF (`0x0C`) e CR (`0x0D`).
#[inline]
pub const fn is_html_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\x0C' | '\r')
}

/// Remove espaços em branco HTML das extremidades da string sem alocação.
pub fn trim_html_whitespace(text: &str) -> &str {
    text.trim_matches(is_html_whitespace)
}

/// Colapsa sequências contíguas de espaços em branco HTML em um único espaço `' '`, conforme as regras de renderização inline CSS.
pub fn collapse_html_whitespace(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_whitespace = false;

    for c in text.chars() {
        if is_html_whitespace(c) {
            if !in_whitespace {
                result.push(' ');
                in_whitespace = true;
            }
        } else {
            result.push(c);
            in_whitespace = false;
        }
    }

    result
}
