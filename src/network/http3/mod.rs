//! HTTP/3 QUIC Client — Implementação Real via quinn + h3
//!
//! Este módulo fornece suporte HTTP/3 real via protocolo QUIC para o Albedo Browser.
//! Vantagens sobre HTTP/2:
//!   - 0-RTT resumption para conexões rápidas (25% mais rápido em cold connections)
//!   - Connection migration (troca de IP/porta não quebra a conexão)
//!   - Stream multiplexing sem head-of-line blocking
//!   - Recuperação de perda de pacotes por stream individual
//!   - Controle de congestionamento per-stream

use bytes::Buf;
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Resposta HTTP/3 com status, headers e corpo

pub mod http3response; pub use http3response::*;
pub mod http3client; pub use http3client::*;
pub mod http3client_impl_1; pub use http3client_impl_1::*;
pub mod http3client_impl_2; pub use http3client_impl_2::*;
pub mod http3client_impl_3; pub use http3client_impl_3::*;
pub mod test_http3_client_creation; pub use test_http3_client_creation::*;
