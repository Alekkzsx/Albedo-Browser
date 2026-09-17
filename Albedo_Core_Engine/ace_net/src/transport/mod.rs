//! # Camada de Transporte Físico HTTP/TLS
//!
//! Gerenciamento de conexões TCP assíncronas, ALPN e negociação TLS.

pub mod client;
pub mod dns;
pub mod timing;

pub use client::TransportClient;
pub use dns::DohHappyEyeballsResolver;
