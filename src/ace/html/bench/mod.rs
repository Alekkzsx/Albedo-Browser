//! Benchmark framework for ACE HTML parser
//! 
//! Provides comprehensive benchmarking capabilities with:
//! - Statistical analysis (mean, median, std dev, percentiles)
//! - Outlier detection using IQR method
//! - Warmup and measurement iterations
//! - Baseline comparison for regression detection
//! - HTML and JSON report generation
//! - CI integration support

pub mod stats;
pub mod runner;
pub mod report;

pub use stats::{BenchStats, StatisticalAnalysis};
pub use runner::{BenchRunner, BenchConfig, BenchResult};
pub use report::{ReportGenerator, ReportFormat};

#[cfg(test)]
mod tests;
