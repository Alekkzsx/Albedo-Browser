//! # Box Tree and Sizing Foundation (`ace_layout::tree`)
//!
//! Dual-tree architecture structural box representation:
//! - `LayoutBox` and `LayoutBoxKind`
//! - `MinMaxSizes` and intrinsic size caching
//! - `ConstraintSpace` and `AvailableSize`
//! - CSS 2.2 §9.2.1.1 anonymous block box wrapping algorithm
//! - `BoxTree` and `BoxTreeBuilder`

pub mod anonymous;
pub mod box_tree;
pub mod constraint_space;
pub mod layout_box;
pub mod min_max;

pub use anonymous::{normalize_table_structure, wrap_anonymous_blocks};
pub use box_tree::{BoxTree, BoxTreeBuilder};
pub use constraint_space::{AvailableSize, ConstraintSpace};
pub use layout_box::{LayoutBox, LayoutBoxKind};
pub use min_max::{MinMaxDependencyFlags, MinMaxSizes, MinMaxSizesCache};
