//! # JIT Contracts Supervisor
//!
//! Camada de contrato do `albedo-jit`.
//! Este modulo e o unico ponto permitido para dependencias "core-like" dentro do JIT.
//! Assim evitamos acoplamento ciclico com o crate `albedo`.

pub mod core {
    use std::collections::HashMap;

    #[derive(Debug, Clone, PartialEq)]
    pub enum JsonValue {
        Null,
        Bool(bool),
        Number(f64),
        String(String),
        Array(Vec<JsonValue>),
        Object(HashMap<String, JsonValue>),
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct Url(pub String);

    #[derive(Debug, Clone, PartialEq)]
    pub enum AlbedoError {
        Json(String),
        Url(String),
        Network(String),
        Jit(String),
        Internal(String),
    }

    pub type AlbedoResult<T> = Result<T, AlbedoError>;

    /// Parser JSON placeholder para manter a API do supervisor estavel no JIT.
    /// Quando o supervisor compartilhado entre crates for extraido, este ponto
    /// deve virar um simples re-export.
    pub fn json_parse(_text: &str) -> AlbedoResult<JsonValue> {
        Err(AlbedoError::Internal(
            "json_parse do supervisor JIT ainda nao foi conectado ao parser oficial".to_string(),
        ))
    }
}

pub mod types {
    pub use super::core::JsonValue;
    pub use super::core::Url;
}

pub mod errors {
    pub use super::core::AlbedoError;
    pub use super::core::AlbedoResult;
}
