//! # Telemetria Atômica e Coleta de Métricas (UMA)
//!
//! Histogramas atômicos lock-free para latência de layout, duração de requisições e taxa de quadros.

pub mod frame;
pub mod histogram;

pub use frame::{FrameBudgetTracker, FrameMetrics, FrameStage};
pub use histogram::{Histogram, HistogramSnapshot};
