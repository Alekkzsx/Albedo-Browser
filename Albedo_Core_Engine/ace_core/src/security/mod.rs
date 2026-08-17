//! # Segurança e Isolamento Web
//!
//! Primitivas canônicas de segurança web, incluindo o modelo de Origens (RFC 6454),
//! Same-Origin Policy (SOP) e políticas de isolamento de contextos.

pub mod origin;

pub use origin::{Host, Origin, Scheme};
