//! # Albedo Core Engine (ACE) - Core Foundation
//!
//! Bem-vindo à camada fundacional do **Albedo Browser**.
//!
//! A crate `ace_core` atua como o sistema nervoso central do navegador, provendo
//! primitivas de identificação, tratamento unificado de erros e orquestração das
//! melhores bibliotecas do ecossistema Rust (como `tokio`, `rayon` e `euclid`).
//!
//! Em vez de reinventar estruturas de dados complexas, o `ace_core` estabelece a
//! padronização de como o restante das camadas (DOM, CSSOM, Layout e Renderização)
//! se comunica e gerencia recursos assíncronos e paralelos.
//!
//! ## Estrutura
//!
//! * [`error`]: Módulo responsável por unificar o tratamento de erros em todo o navegador.
//! * [`id`]: Sistema de tipagem estrita para identificadores globais (newtypes).
//!
//! ## Re-exports Estratégicos
//!
//! Para garantir que todas as camadas do navegador utilizem exatamente a mesma versão das
//! fundações, o `ace_core` re-exporta componentes vitais como o sistema de logging (`tracing`)
//! e o tratamento de resultados (`anyhow`).

pub mod error;
pub mod id;

// Re-exports de crates fundacionais para uso no workspace
pub use anyhow::{Context, Result};
pub use euclid;
pub use smol_str::SmolStr;
pub use tracing::{debug, error, info, trace, warn};
