//! # CSS Grid Formatting Context (GFC)
//!
//! Implementação completa de CSS Grid Layout Level 1:
//! - Dimensionamento de trilhas em grade (`track.rs`): `px`, `%`, `fr`, `auto`, `minmax()`
//! - Resolução de frações flexíveis `fr`
//! - Matriz 2D de ocupação e posicionamento (`placement.rs`): explícito e auto-placement com `grid-auto-flow`
//! - Contexto de formatação em grade (`gfc.rs`): alinhamento de itens, spans e calhas `gap`

pub mod gfc;
pub mod placement;
pub mod track;

pub use gfc::{GridFormattingContext, GridItemResult, GridLayoutResult};
pub use placement::{
    place_grid_items, GridItemPlacement, GridMatrix, GridPlacement, ItemPlacementSpec,
};
pub use track::{resolve_track_sizes, GridTrack, GridTrackBreadth, GridTrackSize};
