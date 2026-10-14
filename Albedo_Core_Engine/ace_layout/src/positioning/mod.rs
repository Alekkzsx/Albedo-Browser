//! # Out-of-Flow Positioning, Sticky & Stacking Contexts
//!
//! Subsystem for absolute positioning, fixed positioning, sticky positioning,
//! 2D affine transforms, containing block resolution, and CSS 2.2 Appendix E
//! 7-layer Stacking Context paint ordering.

pub mod absolute;
pub mod fixed;
pub mod stacking;
pub mod sticky;
pub mod transform;

pub use absolute::{
    establishes_containing_block_for_absolute, resolve_absolute_position, AbsoluteLayoutResult,
    ContainingBlock, ResolvedInsets,
};
pub use fixed::{establishes_containing_block_for_fixed, resolve_fixed_position};
pub use stacking::{
    classify_stacking_layer, establishes_stacking_context, sort_paint_items, PaintItem,
    StackingLayer,
};
pub use sticky::{
    establishes_scrollport, Scrollport, StickyConstraintPositioner, StickyInsets,
};
pub use transform::Transform2D;
