//! # Camada de Transporte Físico HTTP/TLS
//!
//! Gerenciamento de conexões TCP assíncronas, ALPN e negociação TLS.

pub mod client;
pub mod dns;
pub mod timing;
pub mod proxy;

pub use client::TransportClient;
pub use dns::DohHappyEyeballsResolver;
pub use proxy::{
    connect_http_connect_tunnel, connect_socks5_tunnel, establish_connection, ProxyAuth,
    ProxyBypassList, ProxyConfig,
};
