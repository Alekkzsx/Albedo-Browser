//! # Table Formatting Context (TFC)
//!
//! Implementação do algoritmo de layout de tabelas W3C CSS 2.2 §17 e CSS Table Module Level 3:
//! - Coleta e agregação de restrições por coluna (`distribution.rs`)
//! - Distribuição ponderada de largura disponível em 3 regimes (déficit, faixa média, excesso)
//! - Alinhamento vertical interno de células (`tfc.rs`): `top`, `middle`, `bottom`

pub mod distribution;
pub mod tfc;

pub use distribution::{
    aggregate_column_constraints, distribute_column_widths, CellConstraints, ColumnConstraints,
};
pub use tfc::{TableCellResult, TableFormattingContext, TableLayoutResult, TableRowResult};
