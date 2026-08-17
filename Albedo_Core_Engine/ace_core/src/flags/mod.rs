//! # Conjuntos Tipados de Flags
//!
//! Vocabulário unificado de flags binárias para nós DOM, dicas de recálculo de estilo e compositor.

pub mod node;

pub use node::{NodeFlags, RenderFlags, StyleChangeHint};
