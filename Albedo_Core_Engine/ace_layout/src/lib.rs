//! # Albedo Core Engine — ace_layout
//!
//! Subsystem for layout calculation, formatting contexts, box sizing, and fragment generation.

#![forbid(unsafe_code)]

pub mod error;
pub mod tree;

pub mod block;
pub mod inline;

// Stubs for future modules to be implemented in subsequent milestones
pub mod flex {}
pub mod grid {}
pub mod table {}
pub mod positioning {}
pub mod fragment {}
pub mod engine {}

// Re-exports
pub use block::{BfcLayoutResult, BlockFormattingContext, MarginStrut};
pub use error::LayoutError;
pub use inline::{
    BreakOpportunity, FixedTextMeasurer, IfcLayoutResult, InlineFormattingContext, InlineFragment,
    LineBoxFragment, LineBreakClass, LineBreaker, LineSegment, TextMeasurer, TextMetrics,
    VerticalAlign,
};
pub use tree::{
    AvailableSize, BoxTree, BoxTreeBuilder, ConstraintSpace, LayoutBox, LayoutBoxKind,
    MinMaxDependencyFlags, MinMaxSizes, MinMaxSizesCache,
};

