//! # Parser CSS Syntax Level 3 (`ace_style::parser`)
//!
//! Tokenizer streaming acelerado por SIMD, valores de componentes e parser de declarações.

pub mod component_value;
pub mod declaration_parser;
pub mod token;
pub mod tokenizer;

pub use component_value::{ComponentValue, ComponentValueParser, MAX_NESTING_DEPTH};
pub use declaration_parser::{parse_declarations, ParsedDeclaration};
pub use token::Token;
pub use tokenizer::Tokenizer;
