//! # Albedo Core Engine — ace_browser
//!
//! Orquestrador mestre e pipeline integrado do Albedo Browser.
//! Conecta `ace_core`, `ace_ipc`, `ace_net`, `ace_dom` e `ace_style`
//! em um ciclo de vida de renderização web ponta a ponta sem atrito.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod pipeline;

pub use pipeline::{BrowserPipeline, PipelineError};
