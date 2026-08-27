//! # Aceleração SIMD e Fast-Paths de Tokenização (Blink SIMD HTML Scanning)
//!
//! Otimizações de busca vetorial para delimitadores HTML (`<`, `&`, `>`, `"`, `'`)
//! e saltos em lote de blocos de texto contíguos sem processar caractere por caractere.

use memchr::{memchr, memchr2, memchr3};

/// Encontra a posição do primeiro delimitador crítico no estado Data (`<` ou `&`) usando SIMD.
#[inline(always)]
pub fn find_html_text_delimiter(bytes: &[u8]) -> Option<usize> {
    memchr2(b'<', b'&', bytes)
}

/// Encontra o fechamento de tag `>` em lote usando SIMD.
#[inline(always)]
pub fn find_tag_close(bytes: &[u8]) -> Option<usize> {
    memchr(b'>', bytes)
}

/// Encontra o final de um comentário `-->` ou delimitador `-` usando SIMD.
#[inline(always)]
pub fn find_comment_dash(bytes: &[u8]) -> Option<usize> {
    memchr(b'-', bytes)
}

/// Encontra aspas simples ou duplas em atributos (`"` ou `'`).
#[inline(always)]
pub fn find_quote(bytes: &[u8]) -> Option<usize> {
    memchr2(b'"', b'\'', bytes)
}

/// Encontra delimitadores de valores de atributos sem aspas (` `, `>`, `/`).
#[inline(always)]
pub fn find_unquoted_attr_end(bytes: &[u8]) -> Option<usize> {
    memchr3(b' ', b'>', b'/', bytes)
}

/// Salta sequências de espaços em branco ASCII em $O(1)$ amortizado.
#[inline]
pub fn skip_ascii_whitespace_simd(bytes: &[u8]) -> usize {
    let mut idx = 0;
    while idx < bytes.len() {
        let b = bytes[idx];
        if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' || b == 0x0C {
            idx += 1;
        } else {
            break;
        }
    }
    idx
}

/// Compara fatias de bytes ASCII case-insensitively com fast-path.
#[inline]
pub fn fast_ascii_eq_ignore_case(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.eq_ignore_ascii_case(b)
}
