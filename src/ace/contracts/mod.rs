//! # ACE-Contracts (Supervisor)
//!
//! Este módulo atua como o **Supervisor** de integração do Albedo Browser.
//! Ele define o contrato único entre o core da engine e os módulos externos (JIT, Runtime, etc.).
//!
//! ### Regra de Ouro:
//! Nenhum módulo externo deve importar componentes internos do ACE diretamente.
//! Use sempre as exportações consolidadas abaixo.
//!
//! > [!IMPORTANT]
//! > **AVISO DE MANUTENÇÃO**: Este arquivo e seus submódulos serão atualizados
//! > quase sempre que novas funcionalidades de engine forem estabilizadas.

pub mod capabilities;
pub mod errors;
pub mod prelude;
pub mod types;

// Re-exports principais para conveniência
pub use capabilities::{AlbedoCapabilities, ContractVersion};
pub use errors::{AlbedoError, AlbedoResult};
pub use types::*;
