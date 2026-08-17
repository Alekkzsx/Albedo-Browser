//! # Segurança e Isolamento Web
//!
//! Primitivas canônicas de segurança web, incluindo o modelo de Origens (RFC 6454),
//! Same-Origin Policy (SOP), tokens criptográficos de processo (`UnguessableToken`) e isolamento de contextos.

pub mod origin;
pub mod site;
pub mod token;
pub mod utils;

pub use origin::{Host, Origin, Scheme};
pub use site::SchemefulSite;
pub use token::UnguessableToken;
pub use utils::{is_potentially_trustworthy_origin, matches_domain_pattern};
