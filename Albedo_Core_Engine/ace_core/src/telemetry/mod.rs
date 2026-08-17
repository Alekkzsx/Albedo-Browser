//! # Telemetria Atômica e Coleta de Métricas (UMA)
//!
//! Histogramas atômicos lock-free para latência de layout, duração de requisições e taxa de quadros.

pub mod histogram;

pub use histogram::{Histogram, HistogramSnapshot};
