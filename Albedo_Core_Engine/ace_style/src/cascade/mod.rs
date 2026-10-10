//! # Motor de Cascata, Precedência e Especificidade (`ace_style::cascade`)
//!
//! Algoritmo normativo de resolução de precedência CSS conforme CSS Cascade Level 5 & 6.

pub mod layer_order;
pub mod origin;
pub mod sorter;

pub use layer_order::compare_layer_precedence;
pub use origin::{CascadeOrigin, StyleSheetOrigin};
pub use sorter::{CascadeDeclaration, CascadeSorter};
