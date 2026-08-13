// ============================================================================
// Albedo Core Engine (ACE)
// File: lib.rs
// Description: Core module definitions and root exports for the foundation
//              of the browser engine.
// Author: Albedo Browser Engineering Team
// ============================================================================

//! # Albedo Core Engine (`ace_core`)
//!
//! O `ace_core` é o coração absoluto do navegador Albedo.
//! Ele fornece as abstrações primárias sobre as quais todos os outros sistemas
//! (layout, renderização, redes, etc.) são construídos.
//!
//! **Princípios de Design:**
//! - **Zero Dependências:** Código 100% autossuficiente. Nenhuma crate de terceiros é utilizada.
//! - **Performance Crítica:** Todas as abstrações (matemática, I/O, IDs) são desenhadas para o menor overhead (Zero-Cost Abstractions).
//! - **Segurança Absoluta:** Garantia de ausência de UAFs ou overflows via features nativas do Rust.
//!

// ----------------------------------------------------------------------------
// Modules Declaration
// ----------------------------------------------------------------------------

pub mod alloc;
pub mod arena;
pub mod bitset;
pub mod bloom;
pub mod deque;
pub mod ebr;
pub mod error;

// (Esses módulos já haviam sido criados por outra sessão, listamos para compilar tudo)
pub mod event_loop;
pub mod io;
pub mod thread_pool;
pub mod time;

// ----------------------------------------------------------------------------
// Public Exports (Facade)
// ----------------------------------------------------------------------------

pub use bitset::*;
pub use bloom::*;
pub use error::{AceError, AceResult};
pub use event_loop::*;
pub use hash::*;
pub use id::*;
pub use io::*;
pub use log::*;
pub use math::*;
pub use slab::*;
pub use string::*;
pub use thread_pool::*;
pub use time::*;
pub use ebr::*;
pub use partition_alloc::*;
