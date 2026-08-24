//! # Linha do Tempo de Performance W3C
//!
//! Primitivas para perfilamento, marcas e medidas temporais do motor gráfico e APIs web.

pub mod timeline;
pub mod utils;

pub use timeline::{
    PerformanceEntry, PerformanceMark, PerformanceMeasure, PerformanceTimeline, ScopedMeasure,
};
pub use utils::{calculate_percentiles, format_duration_human, Percentiles};
