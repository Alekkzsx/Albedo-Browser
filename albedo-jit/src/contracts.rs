//! # JIT Contracts Supervisor
//!
//! Camada de contrato do `albedo-jit`.
//! Este modulo e o unico ponto permitido para dependencias "core-like" dentro do JIT.
//! Assim evitamos acoplamento ciclico com o crate `albedo`.

pub mod core {
    use std::collections::HashMap;
    use std::sync::OnceLock;

    #[derive(Debug, Clone, PartialEq)]
    pub enum JsonValue {
        Null,
        Bool(bool),
        Number(f64),
        String(String),
        Array(Vec<JsonValue>),
        Object(HashMap<String, JsonValue>),
    }

    impl JsonValue {
        pub fn as_string(&self) -> Option<&str> {
            if let JsonValue::String(s) = self {
                Some(s)
            } else {
                None
            }
        }
        pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
            if let JsonValue::Object(m) = self {
                Some(m)
            } else {
                None
            }
        }
        pub fn as_number(&self) -> Option<f64> {
            if let JsonValue::Number(n) = self {
                Some(*n)
            } else {
                None
            }
        }
        pub fn get(&self, key: &str) -> Option<&JsonValue> {
            if let JsonValue::Object(m) = self {
                m.get(key)
            } else {
                None
            }
        }
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

    static JSON_PARSER: OnceLock<fn(&str) -> Result<JsonValue, String>> = OnceLock::new();

    pub fn register_json_parser(parser: fn(&str) -> Result<JsonValue, String>) {
        let _ = JSON_PARSER.set(parser);
    }

    /// Parser JSON placeholder para manter a API do supervisor estavel no JIT.
    /// Quando o supervisor compartilhado entre crates for extraido, este ponto
    /// deve virar um simples re-export.
    pub fn json_parse(text: &str) -> AlbedoResult<JsonValue> {
        if let Some(parser) = JSON_PARSER.get() {
            parser(text).map_err(|e| AlbedoError::Json(e))
        } else {
            Err(AlbedoError::Internal(
                "json_parse do supervisor JIT ainda nao foi conectado ao parser oficial".to_string(),
            ))
        }
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
