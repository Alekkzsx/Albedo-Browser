pub mod parser;
pub mod serializer;
pub mod tokenizer;
pub mod value;

pub use parser::parse;
pub use serializer::{stringify, stringify_pretty};
pub use value::JsonValue;

#[derive(Debug, PartialEq, Clone)]
pub enum JsonError {
    UnexpectedCharacter(char, usize),
    UnexpectedEndOfInput,
    InvalidNumber(usize),
    InvalidString(usize),
    ExpectedToken(String, usize),
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonError::UnexpectedCharacter(c, pos) => {
                write!(f, "Unexpected character '{}' at position {}", c, pos)
            }
            JsonError::UnexpectedEndOfInput => write!(f, "Unexpected end of input"),
            JsonError::InvalidNumber(pos) => write!(f, "Invalid number at position {}", pos),
            JsonError::InvalidString(pos) => write!(f, "Invalid string at position {}", pos),
            JsonError::ExpectedToken(expected, pos) => {
                write!(f, "Expected {} at position {}", expected, pos)
            }
        }
    }
}

impl std::error::Error for JsonError {}
