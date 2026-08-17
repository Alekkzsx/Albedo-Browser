//! # Cursores de Leitura Sequencial Zero-Copy
//!
//! Abstrações de leitura linear de texto e bytes com suporte a rastreamento de `SourceLocation`.

pub mod byte_cursor;
pub mod char_cursor;
pub mod utils;

pub use byte_cursor::ByteCursor;
pub use char_cursor::CharCursor;
pub use utils::{parse_f32, parse_hex_u32, parse_i32, skip_ascii_whitespace};
