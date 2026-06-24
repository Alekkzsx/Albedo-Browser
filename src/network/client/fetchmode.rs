use super::*;
// ARQUIVO: src/net/fetch.rs

use crate::shared::json::{self, JsonValue};
use crate::network::http3::Http3Client;
use crate::network::security::{AccessControl, Origin};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

// Erros Específicos do Fetch


// Modos de Segurança de Fetch
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchMode {
    Cors,
    NoCors,
    SameOrigin,
}

impl FetchMode {
    /// TODO: add docs
    pub fn to_json(&self) -> JsonValue {
        match self {
            FetchMode::Cors => JsonValue::String("cors".to_string()),
            FetchMode::NoCors => JsonValue::String("no-cors".to_string()),
            FetchMode::SameOrigin => JsonValue::String("same-origin".to_string()),
        }
    }

    /// TODO: add docs
    pub fn from_json(value: &JsonValue) -> Option<Self> {
        match value.as_string()? {
            "cors" => Some(FetchMode::Cors),
            "no-cors" => Some(FetchMode::NoCors),
            "same-origin" => Some(FetchMode::SameOrigin),
            _ => None,
        }
    }
}

impl Default for FetchMode {
pub(crate) fn default() -> Self {
        FetchMode::Cors
    }
}
