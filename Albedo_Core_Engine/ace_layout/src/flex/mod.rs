//! # Flexbox Formatting Context (FFC)
//!
//! Implementação completa de CSS Flexible Box Layout Level 1:
//! - Decomposição de eixos principal e transversal
//! - Resolução de `flex-basis`, `min-width/height` e `max-width/height`
//! - Quebra de linhas com `flex-wrap: nowrap | wrap | wrap-reverse`
//! - Loop de congelamento $O(N)$ (`freeze_loop`) para distribuição elástica
//! - Alinhamento com `justify-content`, `align-items`, `align-self`, `align-content` e calhas `gap`

pub mod algorithm;
pub mod ffc;
pub mod freeze_loop;

pub use algorithm::{
    layout_flex_lines, map_to_logical_rect, split_into_flex_lines, FlexAxis, FlexContainerConfig,
    FlexItem, FlexLine,
};
pub use ffc::{FlexFormattingContext, FlexItemResult, FlexLayoutResult};
pub use freeze_loop::{resolve_flexible_lengths, FlexFreezeItem};
