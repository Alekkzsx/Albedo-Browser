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
pub mod error;
pub mod event_loop;
pub mod id;
pub mod io;
pub mod log;
pub mod math;
pub mod sync;
pub mod thread_pool;
pub mod time;
pub mod trace;

// ----------------------------------------------------------------------------
// Public Exports (Facade)
// ----------------------------------------------------------------------------

pub use error::{AceError, AceResult};
pub use event_loop::*;
pub use id::*;
pub use io::*;
pub use log::*;
pub use math::*;
pub use thread_pool::*;
pub use time::*;
