//! # Layout Engine Master Coordinator (`engine`)
//!
//! Exposes `LayoutEngine`, `LayoutCache`, `LayoutCacheKey`, `LayoutDump`,
//! and Rayon multithreading utilities.

pub mod cache;
pub mod dump;
pub mod layout_engine;
pub mod rayon_parallel;

pub use cache::{LayoutCache, LayoutCacheKey};
pub use dump::LayoutDump;
pub use layout_engine::LayoutEngine;
pub use rayon_parallel::{par_join_subtrees, par_layout_subtrees, RAYON_PARALLEL_THRESHOLD};
