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

#[derive(Debug)]
pub enum FetchError {
    Network(reqwest::Error),
    InvalidUrl(String),
    InvalidMethod,
    BodyError,
    SameOriginBlocked,
    CorsBlocked,
}

impl fmt::Display for FetchError {
pub(crate) fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::Network(err) => write!(f, "Erro de Rede: {}", err),
            FetchError::InvalidUrl(url) => write!(f, "URL Inválida: {}", url),
            FetchError::InvalidMethod => write!(f, "Método HTTP Inválido"),
            FetchError::BodyError => write!(f, "Falha ao processar corpo"),
            FetchError::SameOriginBlocked => {
                write!(f, "Segurança Same-Origin bloqueou a requisição")
            }
            FetchError::CorsBlocked => write!(f, "CORS Bloqueado: Acesso cross-origin negado"),
        }
    }
}

impl std::error::Error for FetchError {}

impl From<reqwest::Error> for FetchError {
pub(crate) fn from(value: reqwest::Error) -> Self {
        FetchError::Network(value)
    }
}
