//! # Layout Error Types (`ace_layout::error`)
//!
//! Error types for the layout engine, box tree building, constraint resolution,
//! and formatting context execution.

use ace_core::id::NodeId;
use thiserror::Error;

/// Erros originados durante a construção da BoxTree, resolução de restrições ou layout.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LayoutError {
    #[error("Root box missing or invalid: {0}")]
    RootNotFound(String),

    #[error("DOM node not found: {0:?}")]
    NodeNotFound(NodeId),

    #[error("Invalid constraint space: {0}")]
    InvalidConstraintSpace(String),

    #[error("Intrinsic sizing calculation failure: {0}")]
    IntrinsicSizingError(String),

    #[error("Unsupported layout format: {0}")]
    UnsupportedLayout(String),

    #[error("Layout engine error: {0}")]
    EngineError(String),

    #[error("Internal layout error: {0}")]
    Internal(String),
}
