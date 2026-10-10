//! # Modelo de Regras e At-Rules CSS (`ace_style::model`)
//!
//! Folhas de estilo, regras de estilo, consultas `@media` e camadas `@layer`.

pub mod layer;
pub mod media;
pub mod stylesheet;

pub use layer::{LayerBlockRule, LayerRegistry, LayerStatementRule};
pub use media::{ColorScheme, MediaCondition, MediaContext, MediaFeature, MediaQuery, MediaRule, MediaType, Orientation};
pub use stylesheet::{CSSRule, ImportRule, KeyframesRule, StyleRule, StyleSheet};
