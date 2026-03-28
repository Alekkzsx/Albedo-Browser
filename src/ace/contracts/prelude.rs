//! # ACE-Contracts: Standard Prelude
//!
//! O cabeçalho padrão para todos os módulos que consomem o contrato de integração.
//! Oferece importação rápida para os tipos estáveis e erros do Supervisor.
//!
//! ### Uso:
//! `use crate::ace::contracts::prelude::*;`
//!
//! > [!IMPORTANT]
//! > **ATUALIZAÇÃO QUASE SEMPRE**: Este arquivo será atualizado sempre que
//! > novos módulos básicos forem integrados ao Supervisor.

pub use super::capabilities::*;
pub use super::errors::*;
pub use super::types::*;

// Utilitários diretos re-exportados no prelude para o JIT/Runtime
pub use crate::ace::json::{parse as json_parse, stringify as json_stringify};
pub use crate::ace::util::base64::{decode as base64_decode, encode as base64_encode};
pub use crate::ace::util::hex::{decode as hex_decode, encode as hex_encode};
pub use crate::ace::util::time::{
    duration_since_epoch, format_http_date, format_iso8601, now as time_now, parse_http_date,
    unix_timestamp, unix_timestamp_millis,
};
