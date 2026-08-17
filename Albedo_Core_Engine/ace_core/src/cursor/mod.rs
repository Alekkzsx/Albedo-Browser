//! # Cursores de Leitura Sequencial Zero-Copy
//!
//! Abstrações de leitura linear de texto e bytes com suporte a rastreamento de `SourceLocation`.

pub mod char_cursor;
pub mod byte_cursor;

pub use char_cursor::CharCursor;
pub use byte_cursor::ByteCursor;
