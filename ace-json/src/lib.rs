pub mod parser;
pub mod serializer;
pub mod tokenizer;
pub mod value;

pub use parser::parse;
pub use serializer::stringify;
pub use value::JsonValue;

#[derive(Debug, PartialEq)]
pub enum JsonError {
    UnexpectedCharacter(char, usize),
    UnexpectedEndOfInput,
    InvalidNumber(usize),
    InvalidString(usize),
    ExpectedToken(String, usize),
}

#[cfg(test)]
mod tests;
