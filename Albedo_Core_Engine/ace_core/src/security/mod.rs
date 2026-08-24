//! # Segurança e Isolamento Web
//!
//! Primitivas canônicas de segurança web, incluindo o modelo de Origens (RFC 6454),
//! Same-Origin Policy (SOP), tokens criptográficos de processo (`UnguessableToken`), Public Suffix List (`CompactPslTrie`)
//! e validação de integridade criptográfica de sub-recursos (`StreamingSriHasher`).

pub mod origin;
pub mod psl;
pub mod referrer;
pub mod site;
pub mod sri;
pub mod token;
pub mod utils;

pub use origin::{Host, Origin, Scheme};
pub use psl::{CompactPslTrie, DomainCategory, PslMatch, RuleType};
pub use referrer::{compute_referrer, ReferrerPolicy};
pub use site::SchemefulSite;
pub use sri::{SriAlgorithm, SriMetadata, StreamingSriHasher};
pub use token::UnguessableToken;
pub use utils::{is_potentially_trustworthy_origin, matches_domain_pattern};
