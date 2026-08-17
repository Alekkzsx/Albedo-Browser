//! # Linha do Tempo de Performance W3C
//!
//! Primitivas para perfilamento, marcas e medidas temporais do motor gráfico e APIs web.

pub mod timeline;

pub use timeline::{
    PerformanceEntry, PerformanceMark, PerformanceMeasure, PerformanceTimeline, ScopedMeasure,
};
