/// Albedo Core Engine (ACE) - Core Foundation
/// 
/// Esta crate fornece os blocos de construção fundamentais para o navegador Albedo.
/// Ela orquestra bibliotecas de nível industrial (`tokio`, `rayon`, `crossbeam`, `euclid`)
/// em uma interface unificada para o resto do motor.

pub mod error;
pub mod id;

// Re-exports de crates fundacionais para uso no workspace
pub use tracing::{info, warn, error, debug, trace};
pub use anyhow::{Result, Context};
pub use euclid;
pub use smol_str::SmolStr;
