//! # Inline Formatting Context (IFC) e Tipografia
//!
//! Submódulo responsável pelo layout em linha, quebra de texto Unicode UAX #14,
//! medição tipográfica, caixas de linha e alinhamento vertical/horizontal.

pub mod ifc;
pub mod line_box;
pub mod line_breaker;
pub mod text_measurer;

// Re-exports públicos
pub use ifc::{IfcLayoutResult, InlineFormattingContext};
pub use line_box::{InlineFragment, LineBoxFragment, VerticalAlign};
pub use line_breaker::{BreakOpportunity, LineBreakClass, LineBreaker, LineSegment};
pub use text_measurer::{FixedTextMeasurer, TextMeasurer, TextMetrics};
