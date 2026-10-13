//! # Out-of-Flow Positioning & Stacking Contexts
//!
//! Subsystem for absolute positioning, fixed positioning, containing block resolution,
//! and CSS 2.2 Appendix E 7-layer Stacking Context paint ordering.

pub mod absolute;
pub mod fixed;
pub mod stacking;

pub use absolute::{
    establishes_containing_block_for_absolute, resolve_absolute_position, AbsoluteLayoutResult,
    ContainingBlock, ResolvedInsets,
};
pub use fixed::{establishes_containing_block_for_fixed, resolve_fixed_position};
pub use stacking::{
    classify_stacking_layer, establishes_stacking_context, sort_paint_items, PaintItem,
    StackingLayer,
};
