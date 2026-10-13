//! # Immutable Fragment Tree Subsystem (`fragment`)
//!
//! Exposes `LayoutFragment`, `FragmentTree`, `LayoutOverflow`, and subpixel pixel snapping.

pub mod fragment_tree;
pub mod layout_fragment;
pub mod overflow;
pub mod pixel_snapping;

pub use fragment_tree::FragmentTree;
pub use layout_fragment::LayoutFragment;
pub use overflow::{union_logical_rects, LayoutOverflow};
pub use pixel_snapping::{snap_box_1d, snap_logical_rect, verify_zero_pixel_cracking};
