//! # Subsistema de Segurança e Políticas Web (CSP & Trusted Types)
//!
//! Blindagem do navegador contra ataques XSS, injeção de scripts e desvio de origens.

pub mod csp;
pub mod trusted_types;

pub use csp::{CSPDirective, CSPPolicy, CSPSource};
pub use trusted_types::{TrustedHTML, TrustedScript, TrustedScriptURL, TrustedTypePolicy};
