//! # ACE-Contracts: Stable Types
//! 
//! Consolidação de tipos primitivos e estruturais da engine ACE.
//! Isola o JIT e o Runtime da localização física dos módulos (JSON, URL, UTIL).
//!
//! > [!IMPORTANT]
//! > **ATUALIZAÇÃO FREQUENTE**: À medida que novos utilitários ACE nativos forem 
//! > desenvolvidos (ex: Hash, Crypto), este arquivo será atualizado.

pub use crate::ace::json::JsonValue;
pub use crate::ace::url::Url;
pub use crate::ace::util::uuid::Uuid;
pub use crate::ace::util::time::{AceTime, DateTimeParts, Weekday};

// Motores de codificação nativos (AFS-29)
pub use crate::ace::util::base64;
pub use crate::ace::util::hex;
pub use crate::ace::util::time;

// Re-exports diretos para API fluída
pub use crate::ace::json::parse as parse_json;
pub use crate::ace::json::stringify as stringify_json;
pub use crate::ace::json::stringify_pretty as stringify_json_pretty;
