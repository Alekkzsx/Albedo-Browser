//! # Albedo Core Engine — ace_layout
//!
//! Subsystem for layout calculation, formatting contexts, box sizing, and fragment generation.

#![forbid(unsafe_code)]

pub mod error;
pub mod tree;

pub mod block;
pub mod inline;

pub mod engine;
pub mod flex;
pub mod fragment;
pub mod grid;
pub mod positioning;
pub mod table;

// Re-exports
pub use block::{BfcLayoutResult, BlockFormattingContext, MarginStrut};
pub use engine::{LayoutCache, LayoutCacheKey, LayoutDump, LayoutEngine};
pub use error::LayoutError;
pub use flex::{
    FlexAxis, FlexContainerConfig, FlexFormattingContext, FlexFreezeItem, FlexItem, FlexItemResult,
    FlexLayoutResult, FlexLine,
};
pub use fragment::{
    snap_box_1d, snap_logical_rect, verify_zero_pixel_cracking, FragmentTree, LayoutFragment,
    LayoutOverflow,
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
pub use positioning::{
    classify_stacking_layer, establishes_containing_block_for_absolute,
    establishes_containing_block_for_fixed, establishes_stacking_context,
    resolve_absolute_position, resolve_fixed_position, sort_paint_items, AbsoluteLayoutResult,
    ContainingBlock, PaintItem, ResolvedInsets, StackingLayer,
};
pub use table::{
    aggregate_column_constraints, distribute_column_widths, CellConstraints, ColumnConstraints,
    TableCellResult, TableFormattingContext, TableLayoutResult, TableRowResult,
};
pub use tree::{
    AvailableSize, BoxTree, BoxTreeBuilder, ConstraintSpace, LayoutBox, LayoutBoxKind,
    MinMaxDependencyFlags, MinMaxSizes, MinMaxSizesCache,
};
