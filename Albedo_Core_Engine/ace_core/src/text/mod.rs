//! # Processamento de Texto e Unicode
//!
//! Primitivas fundamentais para conformidade com a especificação de texto da web,
//! mapeamento de posições UTF-8 (Rust) para UTF-16 (DOM/JS) e normalização de espaços em branco HTML.

pub mod unicode;
pub mod utils;

pub use unicode::{
    collapse_html_whitespace, count_utf16_units, is_html_whitespace, trim_html_whitespace,
    utf16_offset_to_utf8_byte, utf8_byte_to_utf16_offset,
};
pub use utils::{escape_css_identifier, escape_html, is_ascii_case_insensitive_equal};
