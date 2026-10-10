//! # Motor de Estilo e RenderTree (`ace_style::engine`)
//!
//! Coordenação de folhas de estilo, cascata, render tree e cache de compartilhamento de estilo.

pub mod cache;
pub mod style_engine;

pub use cache::{StyleSharingCache, StyleSharingKey};
pub use style_engine::{RenderTree, RenderTreeNode, StyleEngine};
