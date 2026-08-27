//! # Pipeline de Parsing Assíncrono e Background Parser
//!
//! Coordenação de tokenização em thread de fundo, lotes de tokens (`ParsedChunk`) e
//! escalonamento por orçamento de tempo (Time-slicing).

pub mod background;
pub mod chunk;
pub mod scheduler;

pub use background::{BackgroundHTMLParser, BackgroundParserHandle};
pub use chunk::{ParsedChunk, DEFAULT_PARSER_CHUNK_SIZE};
pub use scheduler::{HTMLParserScheduler, DEFAULT_PARSER_BUDGET};
