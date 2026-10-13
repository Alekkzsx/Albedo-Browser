//! # Block Formatting Context (BFC) e Colapso de Margens
//!
//! Submódulo responsável pelo cálculo de dimensionamento de blocos,
//! colapso de margens verticais em $O(1)$ (`MarginStrut`) e contenção de BFCs independentes.

pub mod bfc;
pub mod margin_strut;

// Re-exports públicos
pub use bfc::{BfcLayoutResult, BlockFormattingContext};
pub use margin_strut::MarginStrut;
