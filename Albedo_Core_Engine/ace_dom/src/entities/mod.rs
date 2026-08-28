//! # Resolução e Decodificação de Entidades HTML (Character References)
//!
//! Implementação completa da resolução de referências de caracteres nomeadas e numéricas
//! segundo o padrão WHATWG HTML §12.2.5.72.

pub mod table;

use smol_str::SmolStr;
pub use table::NAMED_ENTITIES;

/// Resolve uma entidade nomeada (ex: `"amp;"` -> `"&"`, `"copy"` -> `"©"`).
#[inline]
pub fn resolve_named_entity(name: &str) -> Option<&'static str> {
    NAMED_ENTITIES.get(name).copied()
}

/// Resolve uma referência numérica de caractere decimal ou hexadecimal.
/// Trata correções normativas de pontos de código legados da tabela Windows-1252 (0x80..=0x9F).
pub fn resolve_numeric_entity(code_point: u32) -> Option<char> {
    // 1. Substituição de códigos inválidos e nulos pelo replacement character
    if code_point == 0 || (0xD800..=0xDFFF).contains(&code_point) || code_point > 0x10FFFF {
        return Some('\u{FFFD}');
    }

    // 2. Mapeamento normativo WHATWG para intervalos legados Windows-1252 (0x80 ..= 0x9F)
    let mapped = match code_point {
        0x80 => 0x20AC, // Euro (€)
        0x82 => 0x201A, // Single low-9 quotation mark (‚)
        0x83 => 0x0192, // Latin small letter f with hook (ƒ)
        0x84 => 0x201E, // Double low-9 quotation mark („)
        0x85 => 0x2026, // Horizontal ellipsis (…)
        0x86 => 0x2020, // Dagger (†)
        0x87 => 0x2021, // Double dagger (‡)
        0x88 => 0x02C6, // Modifier letter circumflex accent (ˆ)
        0x89 => 0x2030, // Per mille sign (‰)
        0x8A => 0x0160, // Latin capital letter S with caron (Š)
        0x8B => 0x2039, // Single left-pointing angle quotation mark (‹)
        0x8C => 0x0152, // Latin capital ligature OE (Œ)
        0x8E => 0x017D, // Latin capital letter Z with caron (Ž)
        0x91 => 0x2018, // Left single quotation mark (‘)
        0x92 => 0x2019, // Right single quotation mark (’)
        0x93 => 0x201C, // Left double quotation mark (“)
        0x94 => 0x201D, // Right double quotation mark (”)
        0x95 => 0x2022, // Bullet (•)
        0x96 => 0x2013, // En dash (–)
        0x97 => 0x2014, // Em dash (—)
        0x98 => 0x02DC, // Small tilde (˜)
        0x99 => 0x2122, // Trade mark sign (™)
        0x9A => 0x0161, // Latin small letter s with caron (š)
        0x9B => 0x203A, // Single right-pointing angle quotation mark (›)
        0x9C => 0x0153, // Latin small ligature oe (œ)
        0x9E => 0x017E, // Latin small letter z with caron (ž)
        0x9F => 0x0178, // Latin capital letter Y with diaeresis (Ÿ)
        other => other,
    };

    char::from_u32(mapped)
}

/// Decodifica uma sequência de entidade completa a partir do corpo (após o `&`).
/// Retorna a string decodificada e a quantidade de caracteres consumidos da fatia.
pub fn decode_character_reference(input: &str) -> Option<(SmolStr, usize)> {
    if input.is_empty() {
        return None;
    }

    let bytes = input.as_bytes();

    // Referência numérica
    if bytes[0] == b'#' {
        if input.len() < 2 {
            return None;
        }

        let is_hex = bytes[1] == b'x' || bytes[1] == b'X';
        let start = if is_hex { 2 } else { 1 };
        let mut end = start;

        while end < input.len() {
            let b = bytes[end];
            if b == b';' {
                break;
            }
            if is_hex {
                if !b.is_ascii_hexdigit() {
                    break;
                }
            } else if !b.is_ascii_digit() {
                break;
            }
            end += 1;
        }

        if end == start {
            return None;
        }

        let num_str = &input[start..end];
        let code_point = if is_hex {
            u32::from_str_radix(num_str, 16).ok()?
        } else {
            num_str.parse::<u32>().ok()?
        };

        let ch = resolve_numeric_entity(code_point)?;
        let has_semicolon = end < input.len() && bytes[end] == b';';
        let consumed = if has_semicolon { end + 1 } else { end };

        let out = SmolStr::new(ch.to_string());
        return Some((out, consumed));
    }

    // Referência nomeada: busca normativa pelo maior prefixo correspondente (WHATWG §12.2.5.73)
    let max_len = input.len().min(32);
    let mut end = 0;
    while end < max_len {
        let b = bytes[end];
        end += 1;
        if b == b';' {
            break;
        }
        if !b.is_ascii_alphanumeric() {
            end -= 1;
            break;
        }
    }

    let mut check_len = end;
    while check_len > 0 {
        let prefix = &input[..check_len];
        if let Some(replacement) = resolve_named_entity(prefix) {
            return Some((SmolStr::new(replacement), check_len));
        }
        check_len -= 1;
    }

    None
}
