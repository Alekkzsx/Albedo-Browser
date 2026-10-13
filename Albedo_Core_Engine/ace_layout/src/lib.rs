//! # Albedo Core Engine — ace_layout
//!
//! Subsystem for layout calculation, formatting contexts, box sizing, and fragment generation.

#![forbid(unsafe_code)]

pub mod error;
pub mod tree;

pub mod block;
pub mod inline;

pub mod flex;
pub mod grid;
pub mod table;
pub mod positioning;
pub mod fragment {}
pub mod engine {}

// Re-exports
pub use block::{BfcLayoutResult, BlockFormattingContext, MarginStrut};
pub use error::LayoutError;
pub use flex::{
    FlexAxis, FlexContainerConfig, FlexFormattingContext, FlexFreezeItem, FlexItem,
    FlexItemResult, FlexLayoutResult, FlexLine,
};
pub use grid::{
    place_grid_items, resolve_track_sizes, GridFormattingContext, GridItemPlacement,
    GridItemResult, GridLayoutResult, GridMatrix, GridPlacement, GridTrack, GridTrackBreadth,
    GridTrackSize, ItemPlacementSpec,
};
pub use inline::{
    BreakOpportunity, FixedTextMeasurer, IfcLayoutResult, InlineFormattingContext, InlineFragment,
    LineBoxFragment, LineBreakClass, LineBreaker, LineSegment, TextMeasurer, TextMetrics,
    VerticalAlign,
};
pub use table::{
    aggregate_column_constraints, distribute_column_widths, CellConstraints, ColumnConstraints,
    TableCellResult, TableFormattingContext, TableLayoutResult, TableRowResult,
};
pub use tree::{
    AvailableSize, BoxTree, BoxTreeBuilder, ConstraintSpace, LayoutBox, LayoutBoxKind,
    MinMaxDependencyFlags, MinMaxSizes, MinMaxSizesCache,
};

