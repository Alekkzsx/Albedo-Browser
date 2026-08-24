//! # Conjuntos Tipados de Flags
//!
//! Vocabulário unificado de flags binárias para nós DOM, dicas de recálculo de estilo e compositor.

pub mod node;
pub mod utils;

pub use node::{NodeFlags, RenderFlags, StyleChangeHint};
pub use utils::{is_node_dirty, style_hint_to_node_flags};
