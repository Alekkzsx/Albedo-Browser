//! # JIT-Contracts Adapter
//! 
//! Este arquivo é o único ponto de entrada para dependências do motor ACE no JIT.
//! Ele atua como um adaptador (bridge), consumindo o Supervisor do core.
//!
//! ### Regra de Ouro:
//! Nunca importe diretamente do `albedo::ace::*`. Use sempre este adaptador.
//!
//! > [!IMPORTANT]
//! > **ATUALIZAÇÃO QUASE SEMPRE**: Este adaptador evoluirá acompanhando 
//! > o contrato oficial do `ace::contracts`.

/// Importação central do contrato supervisor do Core ACE.
/// Em um ambiente de crate único, isso seria `crate::ace::contracts`.
/// Como o JIT é um crate separado no workspace, usamos o re-export do contrato.
pub mod core {
    // Nota: O JIT depende de 'albedo'. Asseguramos que o contrato seja acessível.
    pub use albedo::ace::contracts::prelude::*;
}

/// Tipos básicos traduzidos/adaptados para o domínio do JIT.
pub mod types {
    pub use super::core::JsonValue;
    pub use super::core::Url;
}

/// Sistema de erros unificado para o JIT.
pub mod errors {
    pub use super::core::AlbedoError;
    pub use super::core::AlbedoResult;
}
