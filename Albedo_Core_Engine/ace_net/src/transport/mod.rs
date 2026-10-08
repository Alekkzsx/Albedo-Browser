//! # Camada de Transporte Físico HTTP/TLS
//!
//! Gerenciamento de conexões TCP assíncronas, ALPN e negociação TLS.

pub mod client;
pub mod dns;
pub mod timing;
pub mod proxy;
pub mod network_change;
pub mod dns_cache;

pub use client::TransportClient;
pub use dns::DohHappyEyeballsResolver;
pub use dns_cache::{DnsCacheEntry, PartitionedDnsCache};
pub use network_change::{NetworkChangeEvent, NetworkChangeNotifier, NetworkConnectionType};
pub use proxy::{
    connect_http_connect_tunnel, connect_socks5_tunnel, establish_connection, ProxyAuth,
    ProxyBypassList, ProxyConfig,
};
